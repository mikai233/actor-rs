use std::collections::{HashMap, HashSet};
use std::collections::hash_map::Entry;
use std::fmt::Debug;
use std::sync::RwLockWriteGuard;
use std::time::Duration;

use async_trait::async_trait;
use etcd_client::{GetOptions, KeyValue, PutOptions, WatchOptions};
use tracing::{debug, error, info, trace};

use actor_core::{Actor, DynMessage};
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::props::Props;
use actor_core::actor_ref::{ActorRef, ActorRefExt};
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::ext::etcd_client::EtcdClient;
use actor_core::ext::option_ext::OptionExt;

use crate::cluster::Cluster;
use crate::cluster_daemon::member_keep_alive_failed::MemberKeepAliveFailed;
use crate::cluster_daemon::member_watch_resp::MemberWatchResp;
use crate::cluster_daemon::self_removed::SelfRemoved;
use crate::cluster_event::ClusterEvent;
use crate::etcd_actor::keep_alive::KeepAlive;
use crate::etcd_actor::watch::Watch;
use crate::heartbeat::cluster_heartbeat_receiver::ClusterHeartbeatReceiver;
use crate::heartbeat::cluster_heartbeat_sender::ClusterHeartbeatSender;
use crate::member::{Member, MemberStatus};
use crate::unique_address::UniqueAddress;

pub(crate) mod leave_cluster;
pub(crate) mod add_on_member_up_listener;
pub(crate) mod add_on_member_removed_listener;
mod self_removed;
mod member_keep_alive_failed;
mod member_watch_resp;

#[derive(Debug)]
pub struct ClusterDaemon {
    pub(crate) client: EtcdClient,
    pub(crate) etcd_actor: ActorRef,
    pub(crate) self_addr: UniqueAddress,
    pub(crate) roles: HashSet<String>,
    pub(crate) transport: ActorRef,
    pub(crate) key_addr: HashMap<String, UniqueAddress>,
    pub(crate) cluster: Option<Cluster>,
    pub(crate) members_watch_adapter: ActorRef,
    pub(crate) keep_alive_adapter: ActorRef,
}

#[async_trait]
impl Actor for ClusterDaemon {
    async fn started(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        trace!("{} started", context.myself());
        let cluster = Cluster::get(context.system()).clone();
        self.cluster = Some(cluster);
        context.spawn(
            Props::new(|| Ok(ClusterHeartbeatSender::new())),
            ClusterHeartbeatSender::name(),
        )?;
        context.spawn(
            Props::new(|| Ok(ClusterHeartbeatReceiver::new())),
            ClusterHeartbeatReceiver::name(),
        )?;
        self.watch_cluster_members();
        self.get_all_members(context).await?;
        let lease_id = self.member_keep_alive().await?;
        let member = Member::new(
            self.self_addr.clone(),
            MemberStatus::Up,
            self.roles.clone(),
            lease_id,
        );
        self.update_member_to_etcd(&member).await?;
        Ok(())
    }

    async fn stopped(&mut self, _context: &mut ActorContext) -> anyhow::Result<()> {
        let _ = self.self_removed().await;
        Ok(())
    }
}

impl ClusterDaemon {
    pub(crate) fn new(
        context: &mut ActorContext,
        client: EtcdClient,
        etcd_actor: ActorRef,
        self_addr: UniqueAddress,
        roles: HashSet<String>,
        transport: ActorRef,
    ) -> anyhow::Result<Self> {
        let members_watch_adapter = context.adapter(|m| DynMessage::user(MemberWatchResp(m)));
        let keep_alive_adapter = context.adapter(|m| DynMessage::user(MemberKeepAliveFailed(Some(m))));
        let daemon = Self {
            client,
            etcd_actor,
            self_addr,
            roles,
            transport,
            key_addr: Default::default(),
            cluster: None,
            members_watch_adapter,
            keep_alive_adapter,
        };
        Ok(daemon)
    }

    fn lease_path(&self) -> String {
        format!("actor/{}/cluster/lease", self.self_addr.system_name())
    }

    async fn member_keep_alive(&mut self) -> anyhow::Result<i64> {
        let resp = self.client.lease_grant(60, None).await?;
        let lease_id = resp.id();
        let keep_alive = KeepAlive {
            id: lease_id,
            applicant: self.keep_alive_adapter.clone(),
            interval: Duration::from_secs(3),
        };
        self.etcd_actor.cast_ns(keep_alive);
        Ok(lease_id)
    }

    async fn update_member_to_etcd(&mut self, member: &Member) -> anyhow::Result<()> {
        let socket_addr = member.addr.socket_addr_with_uid();
        let member_addr = socket_addr.as_result()?;
        let lease_path = self.lease_path();
        let key = format!("{}/{}", lease_path, member_addr);
        let value = serde_json::to_vec(&member)?;
        let put_options = PutOptions::new().with_lease(member.lease);
        self.client.put(key, value, Some(put_options)).await?;
        Ok(())
    }

    async fn get_all_members(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        let lease_path = self.lease_path();
        let resp = self.client.get(lease_path, Some(GetOptions::new().with_prefix())).await?;
        for kv in resp.kvs() {
            self.update_local_member_status(context, kv)?;
        }
        Ok(())
    }

    async fn self_removed(&mut self) -> anyhow::Result<()> {
        let self_member = {
            let cluster = self.cluster.as_result_mut()?;
            *cluster.members_write() = HashMap::new();
            let mut self_member = cluster.self_member_write();
            self_member.status = MemberStatus::Removed;
            self_member.clone()
        };
        info!("{} self removed", self_member);
        Ok(())
    }

    fn watch_cluster_members(&mut self) {
        let watch = Watch {
            key: self.lease_path(),
            options: Some(WatchOptions::new().with_prefix()),
            applicant: self.members_watch_adapter.clone(),
        };
        self.etcd_actor.cast_ns(watch);
    }

    async fn try_keep_alive(&mut self, context: &mut ActorContext) {
        match self.member_keep_alive().await {
            Ok(lease_id) => {
                let member = Member::new(
                    self.self_addr.clone(),
                    MemberStatus::Up,
                    self.roles.clone(),
                    lease_id,
                );
                if let Some(error) = self.update_member_to_etcd(&member).await.err() {
                    error!("{} update self member error {:?}, retry it", context.myself(), error);
                    context.myself().cast_ns(MemberKeepAliveFailed(None));
                }
            }
            Err(error) => {
                error!("{} lease error {:?}, retry it", context.myself(), error);
                context.myself().cast_ns(MemberKeepAliveFailed(None));
            }
        }
    }

    fn update_local_member_status(&mut self, context: &mut ActorContext, kv: &KeyValue) -> anyhow::Result<()> {
        let cluster = self.cluster.as_ref().unwrap();
        let stream = context.system().event_stream();
        let member = serde_json::from_slice::<Member>(kv.value())?;
        self.key_addr.insert(kv.key_str()?.to_string(), member.addr.clone());
        debug!("{} update member {}", context.myself(), member);
        if member.addr == self.self_addr {
            *cluster.self_member_write() = member.clone();
        }
        match member.status {
            MemberStatus::Up => {
                if Self::update_member(member.clone(), cluster.members_write()) {
                    stream.publish(ClusterEvent::member_up(member))?;
                }
            }
            MemberStatus::PrepareForLeaving => {
                if Self::update_member(member.clone(), cluster.members_write()) {
                    stream.publish(ClusterEvent::member_prepare_for_leaving(member))?;
                }
            }
            MemberStatus::Leaving => {
                if Self::update_member(member.clone(), cluster.members_write()) {
                    stream.publish(ClusterEvent::member_leaving(member))?;
                }
            }
            MemberStatus::Removed => {
                if cluster.members_write().remove(&member.addr).is_some() {
                    if member.addr == self.self_addr {
                        context.myself().cast_ns(SelfRemoved);
                    }
                    stream.publish(ClusterEvent::member_removed(member))?;
                }
            }
        }
        Ok(())
    }

    fn update_member(member: Member, mut members: RwLockWriteGuard<HashMap<UniqueAddress, Member>>) -> bool {
        match members.entry(member.addr.clone()) {
            Entry::Occupied(mut o) => {
                if &member != o.get() {
                    o.insert(member);
                    true
                } else {
                    false
                }
            }
            Entry::Vacant(v) => {
                v.insert(member);
                true
            }
        }
    }
}

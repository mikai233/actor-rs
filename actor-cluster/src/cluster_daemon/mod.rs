use std::collections::{HashMap, HashSet};
use std::collections::hash_map::Entry;
use std::fmt::Debug;
use std::sync::RwLockWriteGuard;
use std::time::Duration;

use async_trait::async_trait;
use etcd_client::{GetOptions, KeyValue, PutOptions, WatchOptions};
use tracing::{debug, error, info, trace};

use actor_core::{Actor, DynMessage, };
use actor_core::actor::actor_ref::{ActorRef, ActorRefExt};
use actor_core::actor::actor_ref_factory::ActorRefFactory;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::props::Props;
use actor_core::event::EventBus;
use actor_core::ext::etcd_client::EtcdClient;
use actor_core::ext::option_ext::OptionExt;

use crate::cluster::Cluster;
use crate::cluster_daemon::lease_failed::LeaseFailed;
use crate::cluster_daemon::self_removed::SelfRemoved;
use crate::cluster_daemon::watch_resp_wrap::WatchRespWrap;
use crate::cluster_event::ClusterEvent;
use crate::etcd_watcher::EtcdWatcher;
use crate::heartbeat::cluster_heartbeat_receiver::ClusterHeartbeatReceiver;
use crate::heartbeat::cluster_heartbeat_sender::ClusterHeartbeatSender;
use crate::lease_keeper::{EtcdLeaseKeeper, LeaseKeepAliveFailed};
use crate::member::{Member, MemberStatus};
use crate::unique_address::UniqueAddress;

pub(crate) mod leave_cluster;
pub(crate) mod add_on_member_up_listener;
pub(crate) mod add_on_member_removed_listener;
mod self_down;
mod self_removed;
mod lease_failed;
mod watch_resp_wrap;

#[derive(Debug)]
pub struct ClusterDaemon {
    pub(crate) client: EtcdClient,
    pub(crate) self_addr: UniqueAddress,
    pub(crate) roles: HashSet<String>,
    pub(crate) transport: ActorRef,
    pub(crate) key_addr: HashMap<String, UniqueAddress>,
    pub(crate) cluster: Option<Cluster>,
}

#[async_trait]
impl Actor for ClusterDaemon {
    async fn started(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        trace!("{} started", context.myself());
        let cluster = Cluster::get(context.system()).clone();
        self.cluster = Some(cluster);
        context.spawn(
            Props::new_with_ctx(|context| Ok(ClusterHeartbeatSender::new(context))),
            ClusterHeartbeatSender::name(),
        )?;
        context.spawn(
            Props::new_with_ctx(|context| Ok(ClusterHeartbeatReceiver::new(context))),
            ClusterHeartbeatReceiver::name(),
        )?;
        let watcher_adapter = context.message_adapter(|m| DynMessage::user(WatchRespWrap(m)));
        self.spawn_member_watcher(context, watcher_adapter)?;
        self.get_all_members(context).await?;
        let lease_id = self.spawn_lease_keeper(context).await?;
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
        let _ = self.self_down().await;
        Ok(())
    }
}

impl ClusterDaemon {
    fn lease_path(&self) -> String {
        format!("actor/{}/cluster/lease", self.self_addr.system_name())
    }

    fn spawn_watcher(context: &mut ActorContext, name: impl Into<String>, adapter: ActorRef, key: String, options: Option<WatchOptions>, client: EtcdClient) -> anyhow::Result<()> {
        context.spawn(Props::new_with_ctx(move |ctx| {
            Ok(EtcdWatcher::new(
                ctx.myself().clone(),
                client.clone(),
                key.clone(),
                options.clone(),
                adapter.clone(),
            ))
        }), name.into())?;
        Ok(())
    }

    async fn spawn_lease_keeper(&mut self, context: &mut ActorContext) -> anyhow::Result<i64> {
        let resp = self.client.lease_grant(60, None).await?;
        let lease_id = resp.id();
        let client = self.client.clone();
        let receiver = context.message_adapter::<LeaseKeepAliveFailed>(|_| DynMessage::user(LeaseFailed));
        context.spawn(
            Props::new_with_ctx(move |_| { Ok(EtcdLeaseKeeper::new(client.clone(), resp.id(), receiver.clone(), Duration::from_secs(3))) }),
            "lease_keeper",
        )?;
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
        info!("{:?} self removed", self_member);
        Ok(())
    }

    async fn self_down(&mut self) -> anyhow::Result<()> {
        let self_member = {
            let cluster = self.cluster.as_result_mut()?;
            let mut self_member = cluster.self_member_write();
            self_member.status = MemberStatus::Down;
            self_member.clone()
        };
        info!("{:?} self down", self_member);
        Ok(())
    }

    fn spawn_member_watcher(&mut self, context: &mut ActorContext, adapter: ActorRef) -> anyhow::Result<()> {
        Self::spawn_watcher(
            context,
            "member_watcher",
            adapter.clone(),
            self.lease_path(),
            Some(WatchOptions::new().with_prefix()),
            self.client.clone(),
        )?;
        Ok(())
    }

    async fn respawn_lease_keeper(&mut self, context: &mut ActorContext) {
        match self.spawn_lease_keeper(context).await {
            Ok(lease_id) => {
                let member = Member::new(
                    self.self_addr.clone(),
                    MemberStatus::Up,
                    self.roles.clone(),
                    lease_id,
                );
                if let Some(error) = self.update_member_to_etcd(&member).await.err() {
                    error!("{} update self member error {:?}", context.myself(), error);
                    context.myself().cast_ns(LeaseFailed);
                }
            }
            Err(error) => {
                error!("{} spawn lease keeper error {:?}", context.myself(), error);
                context.myself().cast_ns(LeaseFailed);
            }
        }
    }

    fn update_local_member_status(&mut self, context: &mut ActorContext, kv: &KeyValue) -> anyhow::Result<()> {
        let cluster = self.cluster.as_ref().unwrap();
        let stream = context.system().event_stream();
        let member = serde_json::from_slice::<Member>(kv.value())?;
        self.key_addr.insert(kv.key_str()?.to_string(), member.addr.clone());
        debug!("{} update member {:?}", context.myself(), member);
        if member.addr == self.self_addr {
            *cluster.self_member_write() = member.clone();
        }
        match member.status {
            MemberStatus::Up => {
                if Self::update_member(member.clone(), cluster.members_write()) {
                    stream.publish(DynMessage::orphan(ClusterEvent::member_up(member)))?;
                }
            }
            MemberStatus::PrepareForLeaving => {
                if Self::update_member(member.clone(), cluster.members_write()) {
                    stream.publish(DynMessage::orphan(ClusterEvent::member_prepare_for_leaving(member)))?;
                }
            }
            MemberStatus::Leaving => {
                if Self::update_member(member.clone(), cluster.members_write()) {
                    stream.publish(DynMessage::orphan(ClusterEvent::member_leaving(member)))?;
                }
            }
            MemberStatus::Removed => {
                if cluster.members_write().remove(&member.addr).is_some() {
                    if member.addr == self.self_addr {
                        context.myself().cast_ns(SelfRemoved);
                    }
                    stream.publish(DynMessage::orphan(ClusterEvent::member_removed(member)))?;
                }
            }
            MemberStatus::Down => {
                stream.publish(DynMessage::orphan(ClusterEvent::member_downed(member)))?;
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

use std::collections::{HashMap, HashSet};
use std::collections::hash_map::Entry;
use std::ops::Not;
use std::sync::RwLockWriteGuard;
use std::time::Duration;

use anyhow::Context as AnyhowContext;
use async_trait::async_trait;
use etcd_client::{GetOptions, KeyValue, PutOptions, WatchOptions};
use tokio::sync::mpsc::{channel, Sender};
use tracing::{debug, error};

use actor_core::{Actor, DynMessage};
use actor_core::actor::actor_selection::{ActorSelection, ActorSelectionPath};
use actor_core::actor::address::Address;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::coordinated_shutdown::{CoordinatedShutdown, PHASE_CLUSTER_EXITING, PHASE_CLUSTER_EXITING_DONE};
use actor_core::actor_path::root_actor_path::RootActorPath;
use actor_core::actor_path::TActorPath;
use actor_core::actor_ref::{ActorRef, ActorRefExt};
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::ext::etcd_client::EtcdClient;
use actor_core::ext::option_ext::OptionExt;
use actor_core::ext::type_name_of;
use actor_core::pattern::patterns::PatternsExt;
use actor_core::provider::downcast_provider;
use actor_remote::net::tcp_transport::disconnect::Disconnect;

use crate::cluster::Cluster;
use crate::cluster_core_daemon::exiting_completed_req::{ExitingCompletedReq, ExitingCompletedResp};
use crate::cluster_core_daemon::member_keep_alive_failed::MemberKeepAliveFailed;
use crate::cluster_core_daemon::member_watch_resp::MemberWatchResp;
use crate::cluster_core_daemon::self_leaving::SelfLeaving;
use crate::cluster_core_daemon::self_removed::SelfRemoved;
use crate::cluster_event::ClusterEvent;
use crate::cluster_provider::ClusterActorRefProvider;
use crate::etcd_actor::keep_alive::KeepAlive;
use crate::etcd_actor::watch::Watch;
use crate::heartbeat::cluster_heartbeat_sender::ClusterHeartbeatSender;
use crate::member::{Member, MemberStatus};
use crate::unique_address::UniqueAddress;

mod exiting_completed_req;
pub(crate) mod leave;
mod member_keep_alive_failed;
pub mod self_removed;
mod self_leaving;
mod member_watch_resp;

#[derive(Debug)]
pub(crate) struct ClusterCoreDaemon {
    transport: ActorRef,
    key_addr: HashMap<String, UniqueAddress>,
    self_addr: UniqueAddress,
    roles: HashSet<String>,
    client: EtcdClient,
    cluster: Cluster,
    members_watch_adapter: ActorRef,
    keep_alive_adapter: ActorRef,
    self_exiting: Sender<()>,
}

impl ClusterCoreDaemon {
    pub(crate) fn new(context: &mut ActorContext) -> anyhow::Result<Self> {
        let (self_exiting_tx, mut self_exiting_rx) = channel(1);
        let members_watch_adapter = context.adapter(|m| DynMessage::user(MemberWatchResp(m)));
        let keep_alive_adapter = context.adapter(|m| DynMessage::user(MemberKeepAliveFailed(Some(m))));
        let coord_shutdown = CoordinatedShutdown::get(context.system());
        let cluster_ext = Cluster::get(context.system()).clone();
        let cluster = cluster_ext.clone();
        coord_shutdown.add_task(context.system(), PHASE_CLUSTER_EXITING, "wait-exiting", async move {
            if cluster.members().is_empty().not() {
                self_exiting_rx.recv().await;
            }
        })?;
        let myself = context.myself().clone();
        let cluster = cluster_ext.clone();
        let phase_cluster_exiting_done_timeout = CoordinatedShutdown::timeout(context.system(), PHASE_CLUSTER_EXITING_DONE)
            .into_result()
            .context(format!("phase {} not found", PHASE_CLUSTER_EXITING_DONE))?;
        coord_shutdown.add_task(context.system(), PHASE_CLUSTER_EXITING_DONE, "exiting-completed", async move {
            if !(cluster.is_terminated() || cluster.self_member().status == MemberStatus::Removed) {
                if let Some(error) = myself.ask::<_, ExitingCompletedResp>(ExitingCompletedReq, phase_cluster_exiting_done_timeout).await.err() {
                    debug!("ask {} error {:?}", type_name_of::<ExitingCompletedResp>(), error);
                }
            }
        })?;
        let cluster = Cluster::get(context.system()).clone();
        let provider = context.system().provider();
        let cluster_provider = downcast_provider::<ClusterActorRefProvider>(&provider);
        let transport = cluster_provider.remote.transport.clone();
        let self_member = cluster.self_member().clone();
        let self_addr = self_member.addr.clone();
        let roles = self_member.roles.clone();
        let client = cluster.etcd_client();
        let daemon = Self {
            transport,
            key_addr: Default::default(),
            self_addr,
            roles,
            client,
            cluster,
            members_watch_adapter,
            keep_alive_adapter,
            self_exiting: self_exiting_tx,
        };
        Ok(daemon)
    }

    fn cluster_core(context: &mut ActorContext, address: Address) -> anyhow::Result<ActorSelection> {
        let path = RootActorPath::new(address, "/")
            .child("system")
            .child("cluster")
            .child("core")
            .child("daemon");
        context.actor_selection(ActorSelectionPath::FullPath(path))
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
        self.cluster.etcd_actor().cast_ns(keep_alive);
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
        let resp = self.client.get(
            lease_path,
            Some(GetOptions::new().with_prefix()),
        ).await?;
        for kv in resp.kvs() {
            self.update_local_member_status(context, kv)?;
        }
        Ok(())
    }

    fn watch_cluster_members(&mut self) {
        let watch = Watch {
            key: self.lease_path(),
            options: Some(WatchOptions::new().with_prefix()),
            applicant: self.members_watch_adapter.clone(),
        };
        self.cluster.etcd_actor().cast_ns(watch);
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
        let stream = context.system().event_stream();
        let member = serde_json::from_slice::<Member>(kv.value())?;
        self.key_addr.insert(kv.key_str()?.to_string(), member.addr.clone());
        debug!("{} update member {}", context.myself(), member);
        if member.addr == self.self_addr {
            *self.cluster.self_member_write() = member.clone();
        }
        match member.status {
            MemberStatus::Up => {
                if Self::update_member(member.clone(), self.cluster.members_write()) {
                    stream.publish(ClusterEvent::member_up(member))?;
                }
            }
            MemberStatus::PrepareForLeaving => {
                if Self::update_member(member.clone(), self.cluster.members_write()) {
                    stream.publish(ClusterEvent::member_prepare_for_leaving(member))?;
                }
            }
            MemberStatus::Leaving => {
                if member.addr == self.self_addr {
                    context.myself().cast_ns(SelfLeaving);
                }
                if Self::update_member(member.clone(), self.cluster.members_write()) {
                    stream.publish(ClusterEvent::member_leaving(member))?;
                }
            }
            MemberStatus::Removed => {
                if self.cluster.members_write().remove(&member.addr).is_some() {
                    if member.addr == self.self_addr {
                        context.myself().cast_ns(SelfRemoved);
                    } else {
                        self.disconnect_member(&member);
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

    fn disconnect_member(&self, member: &Member) {
        if let Some(addr) = member.addr.socket_addr() {
            let disconnect = Disconnect {
                addr: addr.clone().into(),
            };
            self.transport.cast_ns(disconnect);
        }
    }
}

#[async_trait]
impl Actor for ClusterCoreDaemon {
    async fn started(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        context.spawn(ClusterHeartbeatSender::props(), ClusterHeartbeatSender::name())?;
        self.watch_cluster_members();
        self.get_all_members(context).await?;
        self.try_keep_alive(context).await;
        Ok(())
    }

    async fn stopped(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        let _ = self.self_exiting.send(()).await;
        Ok(())
    }
}

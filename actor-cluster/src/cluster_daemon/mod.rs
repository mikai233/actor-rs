use std::collections::{HashMap, HashSet};
use std::collections::hash_map::Entry;
use std::fmt::Debug;
use std::sync::RwLockWriteGuard;
use std::time::Duration;

use anyhow::Context as AnyhowContext;
use async_trait::async_trait;
use etcd_client::{GetOptions, KeyValue, PutOptions, WatchOptions};
use tokio::sync::mpsc::{channel, Sender};
use tracing::{debug, error, trace};

use actor_core::{Actor, DynMessage};
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::coordinated_shutdown::{ClusterDowningReason, CoordinatedShutdown, PHASE_CLUSTER_LEAVE, PHASE_CLUSTER_SHUTDOWN};
use actor_core::actor::props::Props;
use actor_core::actor_ref::{ActorRef, ActorRefExt};
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::ext::etcd_client::EtcdClient;
use actor_core::ext::option_ext::OptionExt;
use actor_core::ext::type_name_of;
use actor_core::pattern::patterns::PatternsExt;
use actor_remote::net::tcp_transport::disconnect::Disconnect;

use crate::cluster::Cluster;
use crate::cluster_core_supervisor::ClusterCoreSupervisor;
use crate::cluster_daemon::leave_req::LeaveReq;
use crate::cluster_daemon::member_keep_alive_failed::MemberKeepAliveFailed;
use crate::cluster_daemon::member_watch_resp::MemberWatchResp;
use crate::cluster_daemon::self_leaving::SelfLeaving;
use crate::cluster_daemon::self_removed::SelfRemoved;
use crate::cluster_event::ClusterEvent;
use crate::coordinated_shutdown_leave::leave_resp::LeaveResp;
use crate::etcd_actor::keep_alive::KeepAlive;
use crate::etcd_actor::watch::Watch;
use crate::heartbeat::cluster_heartbeat_receiver::ClusterHeartbeatReceiver;
use crate::heartbeat::cluster_heartbeat_sender::ClusterHeartbeatSender;
use crate::member::{Member, MemberStatus};
use crate::unique_address::UniqueAddress;

pub(crate) mod leave;
pub(crate) mod add_on_member_up_listener;
pub(crate) mod add_on_member_removed_listener;
mod self_removed;
mod member_keep_alive_failed;
mod member_watch_resp;
mod leave_req;
mod self_leaving;
pub(crate) mod get_cluster_core_ref_req;

#[derive(Debug)]
pub struct ClusterDaemon {
    client: EtcdClient,
    etcd_actor: ActorRef,
    self_addr: UniqueAddress,
    roles: HashSet<String>,
    transport: ActorRef,
    key_addr: HashMap<String, UniqueAddress>,
    cluster: Option<Cluster>,
    members_watch_adapter: ActorRef,
    keep_alive_adapter: ActorRef,
    core_supervisor: Option<ActorRef>,
    cluster_shutdown: Sender<()>,
}

#[async_trait]
impl Actor for ClusterDaemon {
    async fn started(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        trace!("{} started", context.myself());
        let cluster = Cluster::get(context.system()).clone();
        {
            let cluster = cluster.clone();
            let myself = context.myself().clone();
            let cluster_shutdown = self.cluster_shutdown.clone();
            let coord_shutdown = CoordinatedShutdown::get(context.system());
            let phase_cluster_leave_timeout = coord_shutdown.timeout(PHASE_CLUSTER_LEAVE)
                .into_result()
                .context(format!("phase {} not found", PHASE_CLUSTER_LEAVE))?;
            coord_shutdown.add_task(PHASE_CLUSTER_LEAVE, "leave", async move {
                if cluster.is_terminated() || cluster.self_member().status == MemberStatus::Removed {
                    if let Some(_) = cluster_shutdown.send(()).await.err() {
                        debug!("send shutdown failed because receiver already closed");
                    }
                } else {
                    if let Some(error) = myself.ask::<_, LeaveResp>(LeaveReq, phase_cluster_leave_timeout).await.err() {
                        debug!("ask {} error {:?}", type_name_of::<LeaveReq>(), error);
                    }
                }
            })?;
        }
        self.cluster = Some(cluster);
        context.spawn(
            Props::new(|| Ok(ClusterHeartbeatSender::new())),
            ClusterHeartbeatSender::name(),
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

    async fn stopped(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        let _ = self.cluster_shutdown.send(()).await;
        let system = context.system().clone();
        tokio::spawn(async move {
            let fut = {
                let coord_shutdown = CoordinatedShutdown::get(&system);
                coord_shutdown.run(ClusterDowningReason)
            };
            fut.await;
        });
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
        let (cluster_shutdown_tx, mut cluster_shutdown_rx) = channel(1);
        let coord_shutdown = CoordinatedShutdown::get(context.system());
        coord_shutdown.add_task(PHASE_CLUSTER_SHUTDOWN, "wait_shutdown", async move {
            cluster_shutdown_rx.recv().await;
        })?;
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
            core_supervisor: None,
            cluster_shutdown: cluster_shutdown_tx,
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
                if member.addr == self.self_addr {
                    context.myself().cast_ns(SelfLeaving);
                }
                if Self::update_member(member.clone(), cluster.members_write()) {
                    stream.publish(ClusterEvent::member_leaving(member))?;
                }
            }
            MemberStatus::Removed => {
                if cluster.members_write().remove(&member.addr).is_some() {
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

    fn create_children(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        let core_supervisor = context.spawn(
            Props::new_with_ctx(|ctx| {
                Ok(ClusterCoreSupervisor::new(ctx))
            }),
            "core",
        )?;
        self.core_supervisor = Some(core_supervisor);
        context.spawn(ClusterHeartbeatReceiver::props(), ClusterHeartbeatReceiver::name())?;
        Ok(())
    }
}

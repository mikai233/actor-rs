use std::any::type_name;
use std::collections::hash_map::Entry;
use std::ops::Not;
use std::time::Duration;

use ahash::{HashMap, HashSet};
use anyhow::Context as _;
use async_trait::async_trait;
use etcd_client::{GetOptions, KeyValue, PutOptions, WatchOptions};
use imstr::ImString;
use parking_lot::RwLockWriteGuard;
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
use actor_core::pattern::patterns::PatternsExt;
use actor_core::provider::downcast_provider;
use actor_remote::transport::disconnect::Disconnect;

use crate::cluster::Cluster;
use crate::cluster_core_daemon::exiting_completed_req::{ExitingCompletedReq, ExitingCompletedResp};
use crate::cluster_core_daemon::self_leaving::SelfLeaving;
use crate::cluster_event::ClusterEvent;
use crate::cluster_provider::ClusterActorRefProvider;
use crate::etcd_actor::keep_alive::KeepAlive;
use crate::etcd_actor::watch::Watch;
use crate::heartbeat::cluster_heartbeat_sender::ClusterHeartbeatSender;
use crate::member::{Member, MemberStatus};
use crate::unique_address::UniqueAddress;
use crate::vector_clock::{Node, VectorClock};

mod exiting_completed_req;
pub(crate) mod leave;
pub mod self_removed;
mod self_leaving;

const NUMBER_OF_GOSSIPS_BEFORE_SHUTDOWN_WHEN_LEADER_EXITS: usize = 5;
const MAX_GOSSIPS_BEFORE_SHUTTING_DOWN_MYSELF: usize = 5;
const MAX_TICKS_BEFORE_SHUTTING_DOWN_MYSELF: usize = 4;

#[derive(Debug)]
pub(crate) struct ClusterCoreDaemon {
    transport: ActorRef,
    self_addr: UniqueAddress,
    roles: HashSet<ImString>,
    cluster: Cluster,
    self_dc: String,
    vclock_node: Node,
    self_exiting: Sender<()>,
    exiting_tasks_in_progress: bool,
}

impl ClusterCoreDaemon {
    pub(crate) fn new(context: &mut ActorContext) -> anyhow::Result<Self> {
        let (self_exiting_tx, mut self_exiting_rx) = channel(1);
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
                    debug!("ask {} error {:?}", type_name::<ExitingCompletedResp>(), error);
                }
            }
        })?;
        let cluster = Cluster::get(context.system()).clone();
        let provider = context.system().provider();
        let cluster_provider = downcast_provider::<ClusterActorRefProvider>(&provider);
        let transport = cluster_provider.remote.transport.clone();
        let self_member = cluster.self_member().clone();
        let self_addr = self_member.unique_address.clone();
        let roles = self_member.roles.clone();
        let daemon = Self {
            transport,
            self_addr,
            roles,
            cluster,
            self_dc: "".to_string(),
            vclock_node: (),
            self_exiting: self_exiting_tx,
            exiting_tasks_in_progress: false,
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

    fn self_unique_address(&self) -> &UniqueAddress {
        self.cluster.self_unique_address()
    }

    fn update_member(member: Member, mut members: RwLockWriteGuard<HashMap<UniqueAddress, Member>>) -> bool {
        match members.entry(member.unique_address.clone()) {
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
        if let Some(addr) = member.unique_address.socket_addr() {
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
        Ok(())
    }

    async fn stopped(&mut self, _context: &mut ActorContext) -> anyhow::Result<()> {
        let _ = self.self_exiting.send(()).await;
        Ok(())
    }

    async fn on_recv(&mut self, context: &mut ActorContext, message: DynMessage) -> anyhow::Result<()> {
        Self::handle_message(self, context, message).await
    }
}

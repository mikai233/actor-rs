use std::any::type_name;
use std::collections::hash_map::Entry;
use std::fmt::{Display, Formatter};
use std::ops::{Add, AddAssign, Not, Sub};
use std::time::{Duration, Instant};

use ahash::{HashMap, HashSet};
use anyhow::{anyhow, Context as _};
use async_trait::async_trait;
use bincode::{Decode, Encode};
use imstr::ImString;
use parking_lot::RwLockWriteGuard;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::{channel, Sender};
use tracing::{debug, error};

use actor_core::actor::actor_selection::{ActorSelection, ActorSelectionPath};
use actor_core::actor::address::Address;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::coordinated_shutdown::{
    CoordinatedShutdown, PHASE_CLUSTER_EXITING, PHASE_CLUSTER_EXITING_DONE,
};
use actor_core::actor_path::root_actor_path::RootActorPath;
use actor_core::actor_path::TActorPath;
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::actor_ref::{ActorRef, ActorRefExt};
use actor_core::ext::option_ext::OptionExt;
use actor_core::pattern::patterns::PatternsExt;
use actor_core::util::version::Version;
use actor_core::{Actor, DynMessage};
use actor_remote::artery::disconnect::Disconnect;

use crate::cluster::Cluster;
use crate::cluster_core_daemon::exiting_completed_req::{
    ExitingCompletedReq, ExitingCompletedResp,
};
use crate::cluster_core_daemon::self_leaving::SelfLeaving;
use crate::cluster_event::MemberEvent;
use crate::cluster_provider::ClusterActorRefProvider;
use crate::gossip::Gossip;
use crate::heartbeat::cluster_heartbeat_sender::ClusterHeartbeatSender;
use crate::member::{Member, MemberStatus};
use crate::membership_state::{GossipTargetSelector, MembershipState};
use crate::unique_address::UniqueAddress;
use crate::vector_clock::{Node, VectorClock};

mod exiting_completed_req;
pub(crate) mod leave;
mod self_leaving;
pub mod self_removed;

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
    gossip_target_selector: GossipTargetSelector,
    membership_state: MembershipState,
    is_currently_leader: bool,
    stats_enabled: bool,
    gossip_stats: GossipStats,
    seed_nodes: Vec<Address>,
    seed_node_process: Option<ActorRef>,
    seed_node_process_counter: usize,
    join_seed_nodes_deadline: Option<Instant>,
    leader_action_counter: usize,
    self_down_counter: usize,
    preparing_for_shutdown: bool,
    exiting_tasks_in_progress: bool,
    self_exiting: Sender<()>,
    exiting_confirmed: HashSet<UniqueAddress>,
    later_app_version: Option<Version>,
}

impl ClusterCoreDaemon {
    pub(crate) fn new(context: &mut Context) -> anyhow::Result<Self> {
        let (self_exiting_tx, mut self_exiting_rx) = channel(1);
        let coord_shutdown = CoordinatedShutdown::get(context.system());
        let cluster_ext = Cluster::get(context.system()).clone();
        let cluster = cluster_ext.clone();
        coord_shutdown.add_task(
            context.system(),
            PHASE_CLUSTER_EXITING,
            "wait-exiting",
            async move {
                if cluster.members().is_empty().not() {
                    self_exiting_rx.recv().await;
                }
            },
        )?;
        let myself = context.myself().clone();
        let cluster = cluster_ext.clone();
        let phase_cluster_exiting_done_timeout =
            CoordinatedShutdown::timeout(context.system(), PHASE_CLUSTER_EXITING_DONE)
                .into_result()
                .context(format!("phase {} not found", PHASE_CLUSTER_EXITING_DONE))?;
        coord_shutdown.add_task(
            context.system(),
            PHASE_CLUSTER_EXITING_DONE,
            "exiting-completed",
            async move {
                if !(cluster.is_terminated()
                    || cluster.self_member().status == MemberStatus::Removed)
                {
                    if let Some(error) = myself
                        .ask::<_, ExitingCompletedResp>(
                            ExitingCompletedReq,
                            phase_cluster_exiting_done_timeout,
                        )
                        .await
                        .err()
                    {
                        debug!(
                            "ask {} error {:?}",
                            type_name::<ExitingCompletedResp>(),
                            error
                        );
                    }
                }
            },
        )?;
        let cluster = Cluster::get(context.system()).clone();
        let provider = context.system().provider();
        let cluster_provider = provider.downcast_ref::<ClusterActorRefProvider>()?;
        let transport = cluster_provider.remote.artery.clone();
        let self_member = cluster.self_member().clone();
        let self_addr = self_member.unique_address.clone();
        let roles = self_member.roles.clone();
        let daemon = Self {
            transport,
            self_addr,
            roles,
            cluster,
            self_dc: "".to_string(),
            vclock_node: Node::new(Gossip::vclock_name(cluster.self_unique_address())),
            gossip_target_selector: (),
            membership_state: MembershipState {},
            is_currently_leader: false,
            stats_enabled: false,
            gossip_stats: GossipStats {},
            seed_nodes: vec![],
            seed_node_process: None,
            seed_node_process_counter: 0,
            join_seed_nodes_deadline: None,
            leader_action_counter: 0,
            self_down_counter: 0,
            self_exiting: self_exiting_tx,
            exiting_confirmed: Default::default(),
            exiting_tasks_in_progress: false,
            preparing_for_shutdown: false,
            later_app_version: None,
        };
        Ok(daemon)
    }

    fn latest_gossip(&self) -> &Gossip {
        &self.membership_state.latest_gossip
    }

    fn cluster_core(context: &mut Context, address: Address) -> anyhow::Result<ActorSelection> {
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

    fn update_member(
        member: Member,
        mut members: RwLockWriteGuard<HashMap<UniqueAddress, Member>>,
    ) -> bool {
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
    async fn started(&mut self, context: &mut Context) -> anyhow::Result<()> {
        context.spawn(
            ClusterHeartbeatSender::props(),
            ClusterHeartbeatSender::name(),
        )?;
        Ok(())
    }

    async fn stopped(&mut self, _context: &mut Context) -> anyhow::Result<()> {
        let _ = self.self_exiting.send(()).await;
        Ok(())
    }

    async fn on_recv(&mut self, context: &mut Context, message: DynMessage) -> anyhow::Result<()> {
        Self::handle_message(self, context, message).await
    }
}

#[derive(Debug, Copy, Clone, Hash, Serialize, Deserialize, Encode, Decode)]
pub(crate) struct GossipStats {
    pub(crate) received_gossip_count: i64,
    pub(crate) merge_count: i64,
    pub(crate) same_count: i64,
    pub(crate) newer_count: i64,
    pub(crate) older_count: i64,
}

impl GossipStats {
    pub(crate) fn increment_merge_count(&self) -> GossipStats {
        let mut stats = *self;
        stats.merge_count += 1;
        stats.received_gossip_count += 1;
        stats
    }

    pub(crate) fn increment_same_count(&self) -> GossipStats {
        let mut stats = *self;
        stats.same_count += 1;
        stats.received_gossip_count += 1;
        stats
    }

    pub(crate) fn increment_newer_count(&self) -> GossipStats {
        let mut stats = *self;
        stats.newer_count += 1;
        stats.received_gossip_count += 1;
        stats
    }

    pub(crate) fn increment_older_count(&self) -> GossipStats {
        let mut stats = *self;
        stats.older_count += 1;
        stats.received_gossip_count += 1;
        stats
    }
}

impl Add for GossipStats {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            received_gossip_count: self.received_gossip_count + rhs.received_gossip_count,
            merge_count: self.merge_count + rhs.merge_count,
            same_count: self.same_count + rhs.same_count,
            newer_count: self.newer_count + rhs.newer_count,
            older_count: self.older_count + rhs.older_count,
        }
    }
}

impl Sub for GossipStats {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            received_gossip_count: self.received_gossip_count - rhs.received_gossip_count,
            merge_count: self.merge_count - rhs.merge_count,
            same_count: self.same_count - rhs.same_count,
            newer_count: self.newer_count - rhs.newer_count,
            older_count: self.older_count - rhs.older_count,
        }
    }
}

impl Display for GossipStats {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "GossipStats {{ received_gossip_count: {}, merge_count: {}, same_count: {}, newer_count: {}, older_count: {} }}",
            self.received_gossip_count,
            self.merge_count,
            self.same_count,
            self.newer_count,
            self.older_count)
    }
}

#[derive(Debug, Copy, Clone, Hash, Serialize, Deserialize, Encode, Decode)]
pub(crate) struct VectorClockStats {
    pub(crate) version_size: i32,
    pub(crate) seen_latest: i32,
}

impl VectorClockStats {
    pub(crate) fn new(version_size: i32, seen_latest: i32) -> Self {
        Self {
            version_size,
            seen_latest,
        }
    }
}

impl Display for VectorClockStats {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "VectorClockStats {{ version_size: {}, seen_latest: {} }}",
            self.version_size, self.seen_latest
        )
    }
}

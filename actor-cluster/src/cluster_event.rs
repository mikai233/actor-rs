use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};

use crate::cluster::Cluster;
use crate::cluster_core_daemon::{GossipStats, VectorClockStats};
use crate::member::{Member, MemberStatus};
use crate::membership_state::MembershipState;
use crate::reachability::Reachability;
use crate::unique_address::UniqueAddress;
use actor_core::actor::address::Address;
use actor_core::actor::context::Context;
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::event::event_stream::EventStream;
use ahash::{HashMap, HashSet};
use imstr::ImString;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum SubscriptionInitialStateMode {
    InitialStateAsSnapshot,
    InitialStateAsEvents,
}

pub(crate) trait Sealed {}

impl<T> Sealed for T where T: ClusterDomainEvent {}

pub trait ClusterDomainEvent: Sealed {
    fn name(&self) -> &'static str;
}

macro_rules! impl_cluster_domain_event {
    ($($t:ty),*) => {
        $(
            impl ClusterDomainEvent for $t {
                fn name(&self) -> &'static str {
                    std::any::type_name::<Self>()
                }
            }
        )*
    };
}

impl_cluster_domain_event!(
    MemberJoined,
    MemberWeaklyUp,
    MemberUp,
    MemberLeft,
    MemberPreparingForShutdown,
    MemberReadyForShutdown,
    MemberExited,
    MemberDowned,
    MemberRemoved,
    LeaderChanged,
    RoleLeaderChanged,
    ClusterShuttingDown,
    UnreachableMember,
    ReachableMember,
    UnreachableDataCenter,
    ReachableDataCenter,
    SeenChanged,
    ReachabilityChanged,
    CurrentInternalStats,
    MemberTombstonesChanged
);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentClusterState {
    pub members: BTreeSet<Member>,
    pub unreachable: HashSet<Member>,
    pub seen_by: HashSet<Address>,
    pub leader: Option<Address>,
    pub role_leader_map: HashMap<ImString, Option<Address>>,
    pub unreachable_data_center: HashSet<ImString>,
    pub(crate) member_tombstones: HashSet<UniqueAddress>,
}

impl Display for CurrentClusterState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CurrentClusterState {{ members: [{}], unreachable: [{}], seen_by: [{}], leader: {}, role_leader_map: [{}], unreachable_data_center: [{}], member_tombstones: [{}] }}",
            self.members.iter().join(", "),
            self.unreachable.iter().join(", "),
            self.seen_by.iter().join(", "),
            self.leader.as_ref().map_or("None".to_string(), |addr| addr.to_string()),
            self.role_leader_map.iter().map(|(role, address)| format!("{} => {}", role, address.as_ref().map_or("None".to_string(), |address| address.to_string()))).join(", "),
            self.unreachable_data_center.iter().join(", "),
            self.member_tombstones.iter().join(", "),
        )
    }
}

impl PartialEq for CurrentClusterState {
    fn eq(&self, other: &Self) -> bool {
        self.members == other.members
            && self.unreachable == other.unreachable
            && self.seen_by == other.seen_by
            && self.leader == other.leader
            && self.role_leader_map == other.role_leader_map
    }
}

impl Eq for CurrentClusterState {}

impl Hash for CurrentClusterState {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let unreachable: BTreeSet<_> = self.members.iter().collect();
        let seen_by: BTreeSet<_> = self.seen_by.iter().collect();
        let role_leader_map: BTreeMap<_, _> = self.role_leader_map.iter().collect();
        self.members.hash(state);
        unreachable.hash(state);
        seen_by.hash(state);
        self.leader.hash(state);
        role_leader_map.hash(state);
    }
}

impl CurrentClusterState {
    pub fn role_leader(&self, role: &str) -> Option<&Address> {
        match self.role_leader_map.get(role) {
            Some(Some(address)) => Some(address),
            _ => None,
        }
    }

    pub fn all_roles(&self) -> HashSet<&ImString> {
        self.role_leader_map.keys().collect()
    }

    pub fn all_data_centers(&self) -> HashSet<&str> {
        self.members
            .iter()
            .map(|member| member.data_center())
            .collect()
    }

    pub fn with_unreachable_data_center(
        &self,
        unreachable_data_centers: HashSet<ImString>,
    ) -> Self {
        let mut state = self.clone();
        state.unreachable_data_center = unreachable_data_centers;
        state
    }

    pub fn with_member_tombstones(&self, member_tombstones: HashSet<UniqueAddress>) -> Self {
        let mut state = self.clone();
        state.member_tombstones = member_tombstones;
        state
    }
}

#[derive(Debug)]
pub struct SelfUp {
    pub current_cluster_state: CurrentClusterState,
}

pub trait MemberEvent: ClusterDomainEvent {}

macro_rules! impl_member_event {
    ($($t:ty),*) => {
        $(
            impl MemberEvent for $t {}
        )*
    };
}

impl_member_event!(
    MemberJoined,
    MemberWeaklyUp,
    MemberUp,
    MemberLeft,
    MemberPreparingForShutdown,
    MemberReadyForShutdown,
    MemberExited,
    MemberDowned,
    MemberRemoved
);

#[derive(Debug)]
pub struct MemberJoined {
    pub member: Member,
}

impl MemberJoined {
    pub fn new(member: Member) -> Self {
        assert_eq!(
            member.status,
            MemberStatus::Joining,
            "Expected Joining status, got: {}",
            member.status
        );
        Self { member }
    }
}

impl Display for MemberJoined {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "MemberJoined({})", self.member)
    }
}

#[derive(Debug)]
pub struct MemberWeaklyUp {
    pub member: Member,
}

impl MemberWeaklyUp {
    pub fn new(member: Member) -> Self {
        assert_eq!(
            member.status,
            MemberStatus::WeaklyUp,
            "Expected WeaklyUp status, got: {}",
            member.status
        );
        Self { member }
    }
}

impl Display for MemberWeaklyUp {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "MemberWeaklyUp({})", self.member)
    }
}

#[derive(Debug)]
pub struct MemberUp {
    pub member: Member,
}

impl MemberUp {
    pub fn new(member: Member) -> Self {
        assert_eq!(
            member.status,
            MemberStatus::Up,
            "Expected Up status, got: {}",
            member.status
        );
        Self { member }
    }
}

impl Display for MemberUp {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "MemberUp({})", self.member)
    }
}

#[derive(Debug)]
pub struct MemberLeft {
    pub member: Member,
}

impl MemberLeft {
    pub fn new(member: Member) -> Self {
        assert_eq!(
            member.status,
            MemberStatus::Leaving,
            "Expected Leaving status, got: {}",
            member.status
        );
        Self { member }
    }
}

impl Display for MemberLeft {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "MemberLeft({})", self.member)
    }
}

#[derive(Debug)]
pub struct MemberPreparingForShutdown {
    pub member: Member,
}

impl MemberPreparingForShutdown {
    pub fn new(member: Member) -> Self {
        assert_eq!(
            member.status,
            MemberStatus::PreparingForShutdown,
            "Expected PreparingForShutdown status, got: {}",
            member.status
        );
        Self { member }
    }
}

impl Display for MemberPreparingForShutdown {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "MemberPreparingForShutdown({})", self.member)
    }
}

#[derive(Debug)]
pub struct MemberReadyForShutdown {
    pub member: Member,
}

impl MemberReadyForShutdown {
    pub fn new(member: Member) -> Self {
        assert_eq!(
            member.status,
            MemberStatus::ReadyForShutdown,
            "Expected ReadyForShutdown status, got: {}",
            member.status
        );
        Self { member }
    }
}

impl Display for MemberReadyForShutdown {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "MemberReadyForShutdown({})", self.member)
    }
}

#[derive(Debug)]
pub struct MemberExited {
    pub member: Member,
}

impl MemberExited {
    pub fn new(member: Member) -> Self {
        assert_eq!(
            member.status,
            MemberStatus::Exiting,
            "Expected Exiting status, got: {}",
            member.status
        );
        Self { member }
    }
}

impl Display for MemberExited {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "MemberExited({})", self.member)
    }
}

#[derive(Debug)]
pub struct MemberDowned {
    pub member: Member,
}

impl MemberDowned {
    pub fn new(member: Member) -> Self {
        assert_eq!(
            member.status,
            MemberStatus::Down,
            "Expected Down status, got: {}",
            member.status
        );
        Self { member }
    }
}

impl Display for MemberDowned {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "MemberDowned({})", self.member)
    }
}

#[derive(Debug)]
pub struct MemberRemoved {
    pub member: Member,
    pub previous_status: MemberStatus,
}

impl MemberRemoved {
    pub fn new(member: Member, previous_status: MemberStatus) -> Self {
        assert_eq!(
            member.status,
            MemberStatus::Removed,
            "Expected Removed status, got: {}",
            member.status
        );
        Self {
            member,
            previous_status,
        }
    }
}

impl Display for MemberRemoved {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "MemberRemoved({})", self.member)
    }
}

#[derive(Debug)]
pub struct LeaderChanged {
    pub leader: Option<Address>,
}

impl Display for LeaderChanged {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "LeaderChanged({})",
            self.leader
                .as_ref()
                .map_or("None".to_string(), |addr| addr.to_string())
        )
    }
}

#[derive(Debug)]
pub struct RoleLeaderChanged {
    pub role: String,
    pub leader: Option<Address>,
}

impl Display for RoleLeaderChanged {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RoleLeaderChanged({} => {})",
            self.role,
            self.leader
                .as_ref()
                .map_or("None".to_string(), |addr| addr.to_string())
        )
    }
}

#[derive(Debug)]
pub struct ClusterShuttingDown;

impl Display for ClusterShuttingDown {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "ClusterShuttingDown")
    }
}

pub(crate) trait ReachabilityEvent: ClusterDomainEvent {}

macro_rules! impl_reachability_event {
    ($($t:ty),*) => {
        $(
            impl ReachabilityEvent for $t {}
        )*
    };
}

impl_reachability_event!(UnreachableMember, ReachableMember);

#[derive(Debug)]
pub struct UnreachableMember {
    pub member: Member,
}

#[derive(Debug)]
pub struct ReachableMember {
    pub member: Member,
}

pub trait DataCenterReachabilityEvent: ClusterDomainEvent {}

macro_rules! impl_data_center_reachability_event {
    ($($t:ty),*) => {
        $(
            impl DataCenterReachabilityEvent for $t {}
        )*
    };
}

impl_data_center_reachability_event!(UnreachableDataCenter, ReachableDataCenter);

#[derive(Debug)]
pub struct UnreachableDataCenter {
    pub data_center: ImString,
}

#[derive(Debug)]
pub struct ReachableDataCenter {
    pub data_center: ImString,
}

#[derive(Debug)]
pub(crate) struct SeenChanged {
    pub(crate) convergence: bool,
    pub(crate) seen_by: HashSet<Address>,
}

impl Display for SeenChanged {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SeenChanged {{ convergence: {}, seen_by: [{}] }}",
            self.convergence,
            self.seen_by.iter().join(", ")
        )
    }
}

#[derive(Debug)]
pub(crate) struct ReachabilityChanged {
    pub(crate) reachability: Reachability,
}

impl Display for ReachabilityChanged {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ReachabilityChanged {{ reachability: {} }}",
            self.reachability
        )
    }
}

#[derive(Debug, Copy, Clone, Hash, Serialize, Deserialize)]
pub(crate) struct CurrentInternalStats {
    pub(crate) gossip_stats: GossipStats,
    pub(crate) vclock_stats: VectorClockStats,
}

impl Display for CurrentInternalStats {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CurrentInternalStats {{ gossip_stats: {}, vclock_stats: {} }}",
            self.gossip_stats, self.vclock_stats
        )
    }
}

#[derive(Debug)]
pub(crate) struct MemberTombstonesChanged {
    pub(crate) tombstones: HashSet<UniqueAddress>,
}

impl Display for MemberTombstonesChanged {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "MemberTombstonesChanged {{ tombstones: [{}] }}",
            self.tombstones.iter().join(", ")
        )
    }
}

pub(crate) fn diff_unreachable(
    old_state: &MembershipState,
    new_state: &MembershipState,
) -> Vec<UnreachableMember> {
    let new_gossip = &new_state.latest_gossip;
    let old_reachability = old_state.dc_reachability_no_outside_nodes();
    let old_unreachable_nodes = old_reachability.all_unreachable_or_terminated();
    let new_reachability = new_state.dc_reachability_no_outside_nodes();
    new_reachability
        .all_unreachable_or_terminated()
        .into_iter()
        .filter_map(|node| {
            if !old_unreachable_nodes.contains(&node) && node != &new_state.self_unique_address {
                Some(UnreachableMember {
                    member: new_gossip.member(node).clone(),
                })
            } else {
                None
            }
        })
        .collect()
}

#[derive(Debug)]
pub(crate) struct ClusterDomainEventPublisher {
    cluster: Cluster,
    self_unique_address: UniqueAddress,
    empty_membership_state: MembershipState,
    membership_state: MembershipState,
}

impl Actor for ClusterDomainEventPublisher {
    fn receive(&self) -> Receive<Self> {
        todo!()
    }
}

impl ClusterDomainEventPublisher {
    fn event_stream<'a>(&self, context: &'a Context<Self>) -> &'a EventStream {
        &context.system().event_stream
    }

    fn publish<E: Clone>(&self, context: &Context<Self>, event: E) {
        self.event_stream(context).publish(event);
    }
}

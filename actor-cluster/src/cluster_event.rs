use std::any::Any;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};

use ahash::{HashMap, HashSet};
use anyhow::ensure;
use bincode::{Decode, Encode};
use imstr::ImString;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use actor_core::actor::address::Address;

use crate::cluster_core_daemon::{GossipStats, VectorClockStats};
use crate::member::{Member, MemberStatus};
use crate::reachability::Reachability;
use crate::unique_address::UniqueAddress;

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum SubscriptionInitialStateMode {
    InitialStateAsSnapshot,
    InitialStateAsEvents,
}

pub(crate) trait ClusterDomainEvent {
    fn name(&self) -> &'static str;

    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn into_any(self: Box<Self>) -> Box<dyn Any>;
}

#[macro_export]
macro_rules! impl_cluster_domain_event {
    ($name:ident) => {
        impl ClusterDomainEvent for $name {
            fn name(&self) -> &'static str {
                std::any::type_name::<Self>()
            }

            fn as_any(&self) -> &dyn Any {
                self
            }

            fn as_any_mut(&mut self) -> &mut dyn Any {
                self
            }

            fn into_any(self: Box<Self>) -> Box<dyn Any> {
                self
            }
        }
    };

}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
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
        self.members.iter().map(|member| member.data_center()).collect()
    }

    pub fn with_unreachable_data_center(&self, unreachable_data_centers: HashSet<ImString>) -> Self {
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

pub trait MemberEvent: ClusterDomainEvent {
    fn member(&self) -> &Member;

    fn into_inner(self: Box<Self>) -> Member;
}

#[derive(Debug, Clone)]
pub struct MemberJoined {
    pub member: Member,
}

impl MemberJoined {
    pub fn new(member: Member) -> anyhow::Result<Self> {
        ensure!(
            member.status == MemberStatus::Joining,
            "Member status must be Joining"
        );
        Ok(Self { member })
    }
}

impl_cluster_domain_event!(MemberJoined);

impl MemberEvent for MemberJoined {
    fn member(&self) -> &Member {
        &self.member
    }

    fn into_inner(self: Box<Self>) -> Member {
        self.member
    }
}

impl Display for MemberJoined {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "MemberJoined({})", self.member)
    }
}

#[derive(Debug, Clone)]
pub struct MemberWeaklyUp {
    pub member: Member,
}

impl MemberWeaklyUp {
    pub fn new(member: Member) -> anyhow::Result<Self> {
        ensure!(
            member.status == MemberStatus::WeaklyUp,
            "Member status must be WeaklyUp"
        );
        Ok(Self { member })
    }
}

impl_cluster_domain_event!(MemberWeaklyUp);

impl MemberEvent for MemberWeaklyUp {
    fn member(&self) -> &Member {
        &self.member
    }

    fn into_inner(self: Box<Self>) -> Member {
        self.member
    }
}

impl Display for MemberWeaklyUp {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "MemberWeaklyUp({})", self.member)
    }
}

#[derive(Debug, Clone)]
pub struct MemberUp {
    pub member: Member,
}

impl MemberUp {
    pub fn new(member: Member) -> anyhow::Result<Self> {
        ensure!(member.status == MemberStatus::Up, "Member status must be Up");
        Ok(Self { member })
    }
}

impl_cluster_domain_event!(MemberUp);

impl MemberEvent for MemberUp {
    fn member(&self) -> &Member {
        &self.member
    }

    fn into_inner(self: Box<Self>) -> Member {
        self.member
    }
}

impl Display for MemberUp {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "MemberUp({})", self.member)
    }
}

#[derive(Debug, Clone)]
pub struct MemberLeft {
    pub member: Member,
}

impl MemberLeft {
    pub fn new(member: Member) -> anyhow::Result<Self> {
        ensure!(member.status == MemberStatus::Leaving, "Member status must be Leaving");
        Ok(Self { member })
    }
}

impl_cluster_domain_event!(MemberLeft);

impl MemberEvent for MemberLeft {
    fn member(&self) -> &Member {
        &self.member
    }

    fn into_inner(self: Box<Self>) -> Member {
        self.member
    }
}

impl Display for MemberLeft {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "MemberLeft({})", self.member)
    }
}

#[derive(Debug, Clone)]
pub struct MemberPreparingForShutdown {
    pub member: Member,
}

impl MemberPreparingForShutdown {
    pub fn new(member: Member) -> anyhow::Result<Self> {
        ensure!(
            member.status == MemberStatus::PreparingForShutdown,
            "Member status must be PreparingForShutdown"
        );
        Ok(Self { member })
    }
}

impl_cluster_domain_event!(MemberPreparingForShutdown);

impl MemberEvent for MemberPreparingForShutdown {
    fn member(&self) -> &Member {
        &self.member
    }

    fn into_inner(self: Box<Self>) -> Member {
        self.member
    }
}

impl Display for MemberPreparingForShutdown {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "MemberPreparingForShutdown({})", self.member)
    }
}

#[derive(Debug, Clone)]
pub struct MemberReadyForShutdown {
    pub member: Member,
}

impl MemberReadyForShutdown {
    pub fn new(member: Member) -> anyhow::Result<Self> {
        ensure!(
            member.status == MemberStatus::ReadyForShutdown,
            "Member status must be ReadyForShutdown"
        );
        Ok(Self { member })
    }
}

impl_cluster_domain_event!(MemberReadyForShutdown);

impl MemberEvent for MemberReadyForShutdown {
    fn member(&self) -> &Member {
        &self.member
    }

    fn into_inner(self: Box<Self>) -> Member {
        self.member
    }
}

impl Display for MemberReadyForShutdown {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "MemberReadyForShutdown({})", self.member)
    }
}

#[derive(Debug, Clone)]
pub struct MemberExited {
    pub member: Member,
}

impl MemberExited {
    pub fn new(member: Member) -> anyhow::Result<Self> {
        ensure!(member.status == MemberStatus::Exiting, "Member status must be Exiting");
        Ok(Self { member })
    }
}

impl_cluster_domain_event!(MemberExited);

impl MemberEvent for MemberExited {
    fn member(&self) -> &Member {
        &self.member
    }

    fn into_inner(self: Box<Self>) -> Member {
        self.member
    }
}

impl Display for MemberExited {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "MemberExited({})", self.member)
    }
}

#[derive(Debug, Clone)]
pub struct MemberDowned {
    pub member: Member,
}

impl MemberDowned {
    pub fn new(member: Member) -> anyhow::Result<Self> {
        ensure!(member.status == MemberStatus::Down, "Member status must be Down");
        Ok(Self { member })
    }
}

impl_cluster_domain_event!(MemberDowned);

impl MemberEvent for MemberDowned {
    fn member(&self) -> &Member {
        &self.member
    }

    fn into_inner(self: Box<Self>) -> Member {
        self.member
    }
}

impl Display for MemberDowned {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "MemberDowned({})", self.member)
    }
}

pub trait ReachabilityEvent: ClusterDomainEvent {
    fn member(&self) -> &Member;

    fn into_inner(self: Box<Self>) -> Member;
}

#[derive(Debug, Clone)]
pub struct UnreachableMember {
    pub member: Member,
}

impl_cluster_domain_event!(UnreachableMember);

impl ReachabilityEvent for UnreachableMember {
    fn member(&self) -> &Member {
        &self.member
    }

    fn into_inner(self: Box<Self>) -> Member {
        self.member
    }
}

#[derive(Debug, Clone)]
pub struct ReachableMember {
    pub member: Member,
}

impl_cluster_domain_event!(ReachableMember);

impl ReachabilityEvent for ReachableMember {
    fn member(&self) -> &Member {
        &self.member
    }

    fn into_inner(self: Box<Self>) -> Member {
        self.member
    }
}

pub trait DataCenterReachabilityEvent: ClusterDomainEvent {}

#[derive(Debug, Clone)]
pub struct UnreachableDataCenter {
    pub data_center: ImString,
}

impl_cluster_domain_event!(UnreachableDataCenter);

impl DataCenterReachabilityEvent for UnreachableDataCenter {}

#[derive(Debug, Clone)]
pub struct ReachableDataCenter {
    pub data_center: ImString,
}

impl_cluster_domain_event!(ReachableDataCenter);

impl DataCenterReachabilityEvent for ReachableDataCenter {}

#[derive(Debug)]
pub(crate) struct SeenChanged {
    pub(crate) convergence: bool,
    pub(crate) seen_by: HashSet<Address>,
}

impl_cluster_domain_event!(SeenChanged);

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

impl_cluster_domain_event!(ReachabilityChanged);

impl Display for ReachabilityChanged {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "ReachabilityChanged {{ reachability: {} }}", self.reachability)
    }
}

#[derive(Debug, Copy, Clone, Hash, Serialize, Deserialize, Encode, Decode)]
pub(crate) struct CurrentInternalStats {
    pub(crate) gossip_stats: GossipStats,
    pub(crate) vclock_stats: VectorClockStats,
}

impl_cluster_domain_event!(CurrentInternalStats);

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

impl_cluster_domain_event!(MemberTombstonesChanged);

impl Display for MemberTombstonesChanged {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "MemberTombstonesChanged {{ tombstones: [{}] }}",
            self.tombstones.iter().join(", ")
        )
    }
}
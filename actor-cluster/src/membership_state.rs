use std::collections::BTreeSet;
use std::sync::OnceLock;

use ahash::{HashMap, HashSet, HashSetExt};

use actor_core::ext::maybe_ref::MaybeRef;
use actor_core::hashset;

use crate::gossip::{Gossip, GossipOverview};
use crate::member::{Member, MemberStatus};
use crate::reachability::Reachability;
use crate::unique_address::UniqueAddress;

#[derive(Debug, Clone)]
pub(crate) struct MembershipState {
    latest_gossip: Gossip,
    self_unique_address: UniqueAddress,
    self_dc: String,
    cross_dc_connections: i32,
    self_member: Member,
    dc_reachability: Reachability,
    dc_reachability_without_observations_within: Reachability,
    dc_reachability_excluding_downed_observers: Reachability,
    dc_reachability_no_outside_nodes: Reachability,
    age_sorted_top_oldest_members_per_dc: HashMap<String, BTreeSet<Member>>,
}

impl MembershipState {
    fn members(&self) -> &BTreeSet<Member> {
        &self.latest_gossip.members
    }

    fn overview(&self) -> &GossipOverview {
        &self.latest_gossip.overview
    }

    fn seen(&self) -> MembershipState {
        let mut state = self.clone();
        let gossip = state.latest_gossip.seen(&self.self_unique_address);
        state.latest_gossip = gossip;
        state
    }

    fn convergence(&self, exiting_confirmed: &HashSet<UniqueAddress>) -> bool {
        let first_member_in_dc = !self.members().iter().any(|m| {
            m.data_center == self.self_dc && Self::convergence_member_status().contains(&m.status)
        });

        let member_hindering_convergence_exists = || {
            let member_status = if first_member_in_dc {
                let mut status = Self::convergence_member_status().clone();
                status.insert(MemberStatus::Joining);
                status.insert(MemberStatus::WeaklyUp);
                MaybeRef::Own(status)
            } else {
                MaybeRef::Ref(Self::convergence_member_status())
            };
            self.members().iter().any(|m| {
                (first_member_in_dc || m.data_center == self.self_dc) &&
                    member_status.contains(&m.status) &&
                    !(self.latest_gossip.seen_by_node(&m.unique_address) || exiting_confirmed.contains(&m.unique_address))
            })
        };
        let unreachable_in_dc = self.dc_reachability_excluding_downed_observers
            .all_unreachable_or_terminated()
            .iter()
            .filter(|node| **node != self.self_unique_address && !exiting_confirmed.contains(node))
            .map(|node| self.latest_gossip.members)
    }

    fn leader_member_status() -> &'static HashSet<MemberStatus> {
        static LEADER_MEMBER_STATUS: OnceLock<HashSet<MemberStatus>> = OnceLock::new();
        LEADER_MEMBER_STATUS.get_or_init(|| {
            hashset! {
                MemberStatus::Up,
                MemberStatus::Leaving,
                MemberStatus::PreparingForShutdown,
                MemberStatus::ReadyForShutdown,
            }
        })
    }

    fn convergence_member_status() -> &'static HashSet<MemberStatus> {
        static CONVERGENCE_MEMBER_STATUS: OnceLock<HashSet<MemberStatus>> = OnceLock::new();
        CONVERGENCE_MEMBER_STATUS.get_or_init(|| {
            hashset! {
                MemberStatus::Up,
                MemberStatus::Leaving,
                MemberStatus::PreparingForShutdown,
                MemberStatus::ReadyForShutdown,
            }
        })
    }

    fn convergence_skip_unreachable_with_member_status() -> &'static HashSet<MemberStatus> {
        static CONVERGENCE_SKIP_UNREACHABLE_WITH_MEMBER_STATUS: OnceLock<HashSet<MemberStatus>> = OnceLock::new();
        CONVERGENCE_SKIP_UNREACHABLE_WITH_MEMBER_STATUS.get_or_init(|| {
            hashset! {
                MemberStatus::Down,
                MemberStatus::Exiting,
            }
        })
    }

    pub(crate) fn remove_unreachable_with_member_status() -> &'static HashSet<MemberStatus> {
        static REMOVE_UNREACHABLE_WITH_MEMBER_STATUS: OnceLock<HashSet<MemberStatus>> = OnceLock::new();
        REMOVE_UNREACHABLE_WITH_MEMBER_STATUS.get_or_init(|| {
            hashset! {
                MemberStatus::Down,
                MemberStatus::Exiting,
            }
        })
    }

    fn allowed_to_prepare_to_shutdown() -> &'static HashSet<MemberStatus> {
        static ALLOWED_TO_PREPARE_TO_SHUTDOWN: OnceLock<HashSet<MemberStatus>> = OnceLock::new();
        ALLOWED_TO_PREPARE_TO_SHUTDOWN.get_or_init(|| {
            hashset! {
                MemberStatus::Up,
            }
        })
    }

    fn prepare_for_shutdown_states() -> &'static HashSet<MemberStatus> {
        static PREPARE_FOR_SHUTDOWN_STATES: OnceLock<HashSet<MemberStatus>> = OnceLock::new();
        PREPARE_FOR_SHUTDOWN_STATES.get_or_init(|| {
            hashset! {
                MemberStatus::PreparingForShutdown,
                MemberStatus::ReadyForShutdown,
            }
        })
    }
}
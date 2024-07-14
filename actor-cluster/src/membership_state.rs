use std::collections::BTreeSet;
use std::sync::OnceLock;

use ahash::{HashSet, HashSetExt};

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
            m.data_center() == self.self_dc && Self::convergence_member_status().contains(&m.status)
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
                (first_member_in_dc || m.data_center() == self.self_dc)
                    && member_status.contains(&m.status)
                    && !(self.latest_gossip.seen_by_node(&m.unique_address)
                    || exiting_confirmed.contains(&m.unique_address))
            })
        };
        let mut unreachable_in_dc = self
            .dc_reachability_excluding_downed_observers
            .all_unreachable_or_terminated()
            .iter()
            .filter(|node| **node != self.self_unique_address && !exiting_confirmed.contains(node))
            .map(|node| self.latest_gossip.member(node));
        let all_unreachables_can_be_ignored = unreachable_in_dc.all(|unreachable| {
            Self::convergence_skip_unreachable_with_member_status().contains(&unreachable.status)
        });
        all_unreachables_can_be_ignored && !member_hindering_convergence_exists()
    }

    fn dc_reachability(&self) -> Reachability {
        let nodes = self.members().iter().filter_map(|m| {
            if m.data_center() != self.self_dc {
                Some(&m.unique_address)
            } else {
                None
            }
        }).collect();
        self.overview().reachability.remove_observers(nodes)
    }

    fn dc_reachability_without_observations_within(&self) -> Reachability {
        self.dc_reachability().filter_records(|r| {
            self.latest_gossip.member(&r.subject).data_center() != self.self_dc
        })
    }

    fn dc_reachability_excluding_downed_observers(&self) -> Reachability {
        let members_to_exclude = self.members().iter().filter_map(|m| {
            if matches!(m.status,MemberStatus::Down) || m.data_center() != self.self_dc {
                Some(&m.unique_address)
            } else {
                None
            }
        }).collect();
        let nodes = self.members().iter().filter_map(|m| {
            if m.data_center() != self.self_dc {
                Some(&m.unique_address)
            } else {
                None
            }
        }).collect();
        self.overview().reachability.remove_observers(members_to_exclude).remove()
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
        static CONVERGENCE_SKIP_UNREACHABLE_WITH_MEMBER_STATUS: OnceLock<HashSet<MemberStatus>> =
            OnceLock::new();
        CONVERGENCE_SKIP_UNREACHABLE_WITH_MEMBER_STATUS.get_or_init(|| {
            hashset! {
                MemberStatus::Down,
                MemberStatus::Exiting,
            }
        })
    }

    pub(crate) fn remove_unreachable_with_member_status() -> &'static HashSet<MemberStatus> {
        static REMOVE_UNREACHABLE_WITH_MEMBER_STATUS: OnceLock<HashSet<MemberStatus>> =
            OnceLock::new();
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

#[derive(Debug)]
pub(crate) struct GossipTargetSelector {
    reduce_gossip_different_view_probability: f64,
    cross_dc_gossip_probability: f64,
}

impl GossipTargetSelector {
    fn gossip_target(&self, state: &MembershipState) -> Option<UniqueAddress> {}

    fn gossip_targets(&self, state: &MembershipState) -> Vec<UniqueAddress> {}

    fn random_nodes_for_full_gossip(&self, state: &MembershipState, n: i32) -> Vec<UniqueAddress> {}
}
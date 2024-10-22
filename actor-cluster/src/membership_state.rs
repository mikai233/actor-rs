use std::cmp::min;
use std::collections::hash_map::Entry;
use std::collections::BTreeSet;
use std::sync::OnceLock;

use ahash::{HashMap, HashMapExt, HashSet, HashSetExt};
use rand::seq::SliceRandom;
use rand::{random, thread_rng};

use actor_core::ext::maybe_ref::MaybeRef;
use actor_core::{btree_set, hashset};

use crate::cluster_settings::ClusterSettings;
use crate::gossip::{Gossip, GossipOverview};
use crate::member::{Member, MemberByAgeOrderingRef, MemberStatus};
use crate::reachability::Reachability;
use crate::unique_address::UniqueAddress;

#[derive(Debug, Clone)]
pub(crate) struct MembershipState {
    pub(crate) latest_gossip: Gossip,
    pub(crate) self_unique_address: UniqueAddress,
    pub(crate) self_dc: String,
    pub(crate) cross_dc_connections: usize,
}

impl MembershipState {
    pub(crate) fn self_member(&self) -> MaybeRef<Member> {
        self.latest_gossip.member(&self.self_unique_address)
    }

    pub(crate) fn members(&self) -> &BTreeSet<Member> {
        &self.latest_gossip.members
    }

    pub(crate) fn overview(&self) -> &GossipOverview {
        &self.latest_gossip.overview
    }

    pub(crate) fn seen(&self) -> MembershipState {
        let mut state = self.clone();
        let gossip = state.latest_gossip.seen(&self.self_unique_address);
        state.latest_gossip = gossip;
        state
    }

    pub(crate) fn convergence(&self, exiting_confirmed: &HashSet<UniqueAddress>) -> bool {
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
        let reachability = self.dc_reachability_excluding_downed_observers();
        let mut unreachable_in_dc = reachability
            .all_unreachable_or_terminated()
            .iter()
            .filter(|node| **node != self.self_unique_address && !exiting_confirmed.contains(node))
            .map(|node| self.latest_gossip.member(node));
        let all_unreachables_can_be_ignored = unreachable_in_dc.all(|unreachable| {
            Self::convergence_skip_unreachable_with_member_status().contains(&unreachable.status)
        });
        all_unreachables_can_be_ignored && !member_hindering_convergence_exists()
    }

    pub(crate) fn dc_reachability(&self) -> Reachability {
        let nodes = self
            .members()
            .iter()
            .filter_map(|m| {
                if m.data_center() != self.self_dc {
                    Some(&m.unique_address)
                } else {
                    None
                }
            })
            .collect();
        self.overview().reachability.remove_observers(nodes)
    }

    pub(crate) fn dc_reachability_without_observations_within(&self) -> Reachability {
        self.dc_reachability()
            .filter_records(|r| self.latest_gossip.member(&r.subject).data_center() != self.self_dc)
    }

    pub(crate) fn dc_reachability_excluding_downed_observers(&self) -> Reachability {
        let members_to_exclude = self
            .members()
            .iter()
            .filter_map(|m| {
                if matches!(m.status, MemberStatus::Down) || m.data_center() != self.self_dc {
                    Some(&m.unique_address)
                } else {
                    None
                }
            })
            .collect();
        let nodes = self.members().iter().filter_map(|m| {
            if m.data_center() != self.self_dc {
                Some(&m.unique_address)
            } else {
                None
            }
        });
        self.overview()
            .reachability
            .remove_observers(members_to_exclude)
            .remove(nodes)
    }

    pub(crate) fn dc_reachability_no_outside_nodes(&self) -> Reachability {
        let nodes = self.members().iter().filter_map(|m| {
            if m.data_center() != self.self_dc {
                Some(&m.unique_address)
            } else {
                None
            }
        });
        self.overview().reachability.remove(nodes)
    }

    pub(crate) fn age_sorted_top_oldest_members_per_dc(
        &self,
    ) -> HashMap<&str, BTreeSet<MemberByAgeOrderingRef>> {
        self.latest_gossip.members.iter().fold(
            HashMap::<&str, BTreeSet<MemberByAgeOrderingRef>>::new(),
            |mut acc, member| {
                match acc.entry(member.data_center()) {
                    Entry::Occupied(mut o) => {
                        let members = o.get_mut();
                        if members.len() < self.cross_dc_connections {
                            members.insert(member.into());
                        } else {
                            if members.iter().any(|m| member.is_older_than(m)) {
                                members.insert(member.into());
                                while members.len() > self.cross_dc_connections {
                                    members.pop_last();
                                }
                            }
                        }
                    }
                    Entry::Vacant(v) => {
                        v.insert(btree_set!(member.into()));
                    }
                }
                acc
            },
        )
    }

    pub(crate) fn is_reachable_excluding_downed_observers(
        &self,
        to_address: &UniqueAddress,
    ) -> bool {
        if !self.latest_gossip.has_member(to_address) {
            false
        } else {
            let to = self.latest_gossip.member(to_address);
            if self.self_dc == to.data_center() {
                self.dc_reachability_excluding_downed_observers()
                    .is_reachable_by_node(to_address)
            } else {
                self.latest_gossip
                    .reachability_excluding_downed_observers()
                    .is_reachable_by_node(to_address)
            }
        }
    }

    pub(crate) fn dc_members(&self) -> BTreeSet<&Member> {
        if self.latest_gossip.is_multi_dc() {
            self.members()
                .iter()
                .filter(|m| m.data_center() == self.self_dc)
                .collect()
        } else {
            self.members().iter().collect()
        }
    }

    pub(crate) fn is_leader(&self, node: &UniqueAddress) -> bool {
        self.leader().map_or(false, |leader| leader == node)
    }

    pub(crate) fn leader(&self) -> Option<&UniqueAddress> {
        self.leader_of(self.members().iter().collect())
    }

    pub(crate) fn role_leader(&self, role: &str) -> Option<&UniqueAddress> {
        self.leader_of(self.members().iter().filter(|m| m.has_role(role)).collect())
    }

    pub(crate) fn leader_of<'a>(&self, members: BTreeSet<&'a Member>) -> Option<&'a UniqueAddress> {
        let reachability = self.dc_reachability();
        let reachable_members_in_dc: BTreeSet<_> = if reachability.is_all_unreachable() {
            members
                .into_iter()
                .filter(|m| {
                    m.data_center() == self.self_dc && !matches!(m.status, MemberStatus::Down)
                })
                .collect()
        } else {
            members
                .into_iter()
                .filter(|m| {
                    m.data_center() == self.self_dc
                        && !matches!(m.status, MemberStatus::Down)
                        && (reachability.is_reachable_by_node(&m.unique_address)
                            || m.unique_address == self.self_unique_address)
                })
                .collect()
        };
        if reachable_members_in_dc.is_empty() {
            None
        } else {
            reachable_members_in_dc
                .iter()
                .find(|m| Self::leader_member_status().contains(&m.status))
                .or_else(|| {
                    reachable_members_in_dc
                        .iter()
                        .min_by(|a, b| Member::leader_status_ordering(a, b))
                })
                .map(|m| &m.unique_address)
        }
    }

    pub(crate) fn is_in_same_dc(&self, node: &UniqueAddress) -> bool {
        *node == self.self_unique_address
            || self.latest_gossip.member(node).data_center() == self.self_dc
    }

    pub(crate) fn valid_node_for_gossip(&self, node: &UniqueAddress) -> bool {
        *node == self.self_unique_address
            && self
                .overview()
                .reachability
                .is_reachable(&self.self_unique_address, node)
    }

    pub(crate) fn youngest_member(&self) -> &Member {
        let members = self.dc_members();
        assert!(!members.is_empty(), "No youngest when no members");
        members
            .iter()
            .max_by_key(|m| {
                if m.up_number == i32::MAX {
                    0
                } else {
                    m.up_number
                }
            })
            .unwrap()
    }

    pub(crate) fn gossip_targets_for_exiting_members(
        &self,
        exiting_members: HashSet<&Member>,
    ) -> HashSet<&Member> {
        if !exiting_members.is_empty() {
            let roles: HashSet<_> = exiting_members
                .into_iter()
                .flat_map(|m| &m.roles)
                .filter(|role| !role.starts_with(ClusterSettings::dc_role_prefix()))
                .collect();
            let members_sorted_by_age: BTreeSet<_> = self
                .latest_gossip
                .members
                .iter()
                .filter(|m| m.data_center() == self.self_dc)
                .map(MemberByAgeOrderingRef)
                .collect();
            let mut targets = HashSet::new();
            if !members_sorted_by_age.is_empty() {
                let mut iter = members_sorted_by_age.clone().into_iter();
                targets.insert(iter.next().unwrap());
                if let Some(second_oldest) = iter.next() {
                    targets.insert(second_oldest);
                }
                for role in roles {
                    members_sorted_by_age
                        .clone()
                        .into_iter()
                        .find(|m| m.has_role(&role))
                        .into_foreach(|first| {
                            targets.insert(first.clone());
                            members_sorted_by_age
                                .clone()
                                .into_iter()
                                .find(|m| *m != first && m.has_role(&role))
                                .into_foreach(|next| {
                                    targets.insert(next);
                                });
                        });
                }
            }
            targets.into_iter().map(|m| m.into()).collect()
        } else {
            hashset!()
        }
    }

    pub(crate) fn leader_member_status() -> &'static HashSet<MemberStatus> {
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

    pub(crate) fn convergence_member_status() -> &'static HashSet<MemberStatus> {
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

    pub(crate) fn convergence_skip_unreachable_with_member_status() -> &'static HashSet<MemberStatus>
    {
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

    pub(crate) fn allowed_to_prepare_to_shutdown() -> &'static HashSet<MemberStatus> {
        static ALLOWED_TO_PREPARE_TO_SHUTDOWN: OnceLock<HashSet<MemberStatus>> = OnceLock::new();
        ALLOWED_TO_PREPARE_TO_SHUTDOWN.get_or_init(|| {
            hashset! {
                MemberStatus::Up,
            }
        })
    }

    pub(crate) fn prepare_for_shutdown_states() -> &'static HashSet<MemberStatus> {
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
    pub(crate) reduce_gossip_different_view_probability: f64,
    pub(crate) cross_dc_gossip_probability: f64,
}

impl GossipTargetSelector {
    pub(crate) fn gossip_target(&self, state: &MembershipState) -> Option<UniqueAddress> {
        unimplemented!()
    }

    pub(crate) fn gossip_targets(&self, state: &MembershipState) -> Vec<UniqueAddress> {
        unimplemented!()
    }

    pub(crate) fn random_nodes_for_full_gossip(
        &self,
        state: &MembershipState,
        n: usize,
    ) -> Vec<UniqueAddress> {
        if state.latest_gossip.is_multi_dc()
            && state
                .age_sorted_top_oldest_members_per_dc()
                .get(&*state.self_dc)
                .map_or(false, |members| {
                    members.contains(&MemberByAgeOrderingRef(&*state.self_member()))
                })
        {
            let mut random = thread_rng();
            let mut random_local_nodes: Vec<_> = state
                .members()
                .iter()
                .filter_map(|m| {
                    if m.data_center() == state.self_dc
                        && state.valid_node_for_gossip(&m.unique_address)
                    {
                        Some(&m.unique_address)
                    } else {
                        None
                    }
                })
                .collect();
            random_local_nodes.shuffle(&mut random);

            fn select_other_dc_node(
                state: &MembershipState,
                randomized_dcs: &[&str],
            ) -> Option<UniqueAddress> {
                match randomized_dcs {
                    [] => None,
                    [dc, tail @ ..] => {
                        let members = state.age_sorted_top_oldest_members_per_dc();
                        match members
                            .get(dc)
                            .and_then(|members| members.first().map(|m| m.unique_address.clone()))
                        {
                            Some(addr) => Some(addr),
                            None => select_other_dc_node(state, tail),
                        }
                    }
                }
            }
            let mut other_dcs: Vec<_> = state
                .age_sorted_top_oldest_members_per_dc()
                .into_keys()
                .collect();
            other_dcs.retain(|d| *d != state.self_dc);

            match select_other_dc_node(state, &other_dcs) {
                Some(node) => {
                    let mut nodes: Vec<_> = random_local_nodes
                        .into_iter()
                        .take(min(0, n - 1))
                        .cloned()
                        .collect();
                    nodes.push(node);
                    nodes
                }
                None => random_local_nodes.into_iter().take(n).cloned().collect(),
            }
        } else {
            let mut selected_nodes: Vec<_> = state
                .members()
                .iter()
                .filter_map(|m| {
                    if m.data_center() == state.self_dc
                        && state.valid_node_for_gossip(&m.unique_address)
                    {
                        Some(m.unique_address.clone())
                    } else {
                        None
                    }
                })
                .collect();
            if selected_nodes.len() <= n {
                selected_nodes
            } else {
                let mut random = thread_rng();
                selected_nodes.shuffle(&mut random);
                selected_nodes.into_iter().take(n).collect()
            }
        }
    }

    pub(crate) fn local_dc_gossip_targets(&self, state: &MembershipState) -> Vec<UniqueAddress> {
        let latest_gossip = &state.latest_gossip;
        unimplemented!()
    }

    pub(crate) fn multi_dc_gossip_targets(&self, state: &MembershipState) -> Vec<UniqueAddress> {
        unimplemented!()
    }

    pub(crate) fn adjusted_gossip_different_view_probability(&self, cluster_size: usize) -> f64 {
        let low = self.reduce_gossip_different_view_probability;
        let high = low * 3.0;
        if cluster_size as f64 <= low {
            self.reduce_gossip_different_view_probability
        } else {
            let min_p = self.reduce_gossip_different_view_probability / 10.0;
            if cluster_size as f64 >= high {
                min_p
            } else {
                let k = (min_p - self.reduce_gossip_different_view_probability) / (high - low);
                self.reduce_gossip_different_view_probability + (cluster_size as f64 - low) * k
            }
        }
    }

    pub(crate) fn select_dc_local_nodes(&self, state: &MembershipState) -> bool {
        let local_members = state.dc_members().len();
        let probability = if local_members > 4 {
            self.cross_dc_gossip_probability
        } else {
            self.cross_dc_gossip_probability
                .max((5.0 - local_members as f64) * 0.25)
        };
        random::<f64>() > probability
    }

    pub(crate) fn prefer_nodes_with_different_view(&self, state: &MembershipState) -> bool {
        random::<f64>()
            < self.adjusted_gossip_different_view_probability(state.latest_gossip.members.len())
    }

    pub(crate) fn dcs_in_random_order<'a>(&self, mut dcs: Vec<&'a str>) -> Vec<&'a str> {
        dcs.shuffle(&mut thread_rng());
        dcs
    }

    pub(crate) fn select_random_node<'a, I>(
        &self,
        nodes: impl Iterator<Item = &'a UniqueAddress>,
    ) -> Option<&'a UniqueAddress> {
        rand::seq::IteratorRandom::choose(nodes, &mut thread_rng())
    }
}

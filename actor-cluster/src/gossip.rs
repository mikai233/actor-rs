use std::collections::BTreeSet;
use std::fmt::Display;
use std::ops::Add;
use std::time::Duration;

use ahash::{HashMap, HashSet};
use itertools::Itertools;
use sha2::{Digest, Sha256};

use actor_core::ext::maybe_ref::MaybeRef;
use actor_core::hashset;

use crate::member::{Member, MemberStatus};
use crate::reachability::Reachability;
use crate::unique_address::UniqueAddress;
use crate::vector_clock::{Node, VectorClock};

#[derive(Debug, Clone)]
pub(crate) struct Gossip {
    pub(crate) members: BTreeSet<Member>,
    pub(crate) overview: GossipOverview,
    version: VectorClock,
    tombstones: HashMap<UniqueAddress, i64>,
}

impl Gossip {
    pub(crate) fn new(
        members: BTreeSet<Member>,
        overview: GossipOverview,
        version: VectorClock,
        tombstones: HashMap<UniqueAddress, i64>,
    ) -> Self {
        let gossip = Self {
            members,
            overview,
            version,
            tombstones,
        };
        gossip
    }

    fn assert_invariants(&self) {
        assert!(
            !self
                .members
                .iter()
                .any(|m| matches!(m.status, MemberStatus::Removed)),
            "Live members must not have status [{}]",
            MemberStatus::Removed
        );

        let in_reachability_but_not_member = self
            .overview
            .reachability
            .all_observers()
            .difference(&self.members_map().into_keys().collect::<HashSet<_>>())
            .collect::<HashSet<_>>();
        assert!(
            in_reachability_but_not_member.is_empty(),
            "Nodes not part of cluster in reachability table"
        );

        let seen_but_not_member = self
            .overview
            .seen
            .difference(&self.members.iter().map(|m| m.unique_address.clone()).collect())
            .collect_vec();
        assert!(
            seen_but_not_member.is_empty(),
            "Nodes not part of cluster have marked the Gossip as seen"
        );
    }

    pub(crate) fn members_map(&self) -> HashMap<&UniqueAddress, &Member> {
        self.members
            .iter()
            .map(|m| (&m.unique_address, m))
            .collect()
    }

    pub(crate) fn is_multi_dc(&self) -> bool {
        if self.members.len() <= 1 {
            true
        } else {
            let dc1 = self.members.first().unwrap();
            self.members
                .iter()
                .any(|m| m.data_center() != dc1.data_center())
        }
    }

    pub(crate) fn reachability_excluding_downed_observers(&self) -> Reachability {
        let downed = self
            .members
            .iter()
            .filter(|m| matches!(m.status, MemberStatus::Down))
            .map(|m| &m.unique_address)
            .collect();
        self.overview.reachability.remove_observers(downed)
    }

    pub(crate) fn vclock_name(node: &UniqueAddress) -> String {
        format!("{}-{}", node.address, node.uid)
    }

    pub(crate) fn seen(&self, node: &UniqueAddress) -> Gossip {
        if self.seen_by_node(node) {
            self.clone()
        } else {
            let mut gossip = self.clone();
            gossip.overview.seen.insert(node.clone());
            gossip
        }
    }

    fn only_seen(&self, node: UniqueAddress) -> Gossip {
        let mut gossip = self.clone();
        gossip.overview.seen = hashset!(node);
        gossip
    }

    fn clear_seen(&self) -> Gossip {
        let mut gossip = self.clone();
        gossip.overview.seen.clear();
        gossip
    }

    fn seen_by(&self) -> &HashSet<UniqueAddress> {
        &self.overview.seen
    }

    pub(crate) fn seen_by_node(&self, node: &UniqueAddress) -> bool {
        self.overview.seen.contains(node)
    }

    fn merge_seen(&self, that: &Gossip) -> Gossip {
        let mut gossip = self.clone();
        gossip
            .overview
            .seen
            .extend(that.overview.seen.iter().cloned());
        gossip
    }

    fn merge(&self, that: &Gossip) -> Gossip {
        let mut merged_tombstones = self.tombstones.clone();
        merged_tombstones.extend(that.tombstones.clone().into_iter());

        let merged_vclock =
            merged_tombstones
                .keys()
                .fold(self.version.merge(&that.version), |vclock, node| {
                    let node = Node::new(Gossip::vclock_name(&node));
                    vclock.prune(&node)
                });

        let merged_members = Member::pick_highest_priority_with_tombstones(
            self.members.iter().collect(),
            that.members.iter().collect(),
            &merged_tombstones,
        );

        let allowed = merged_members
            .iter()
            .map(|m| &m.unique_address)
            .cloned()
            .collect::<HashSet<_>>();
        let merged_reachability = self
            .overview
            .reachability
            .merge(&allowed, &that.overview.reachability);

        let merged_seen = hashset!();

        let overview = GossipOverview {
            seen: merged_seen,
            reachability: merged_reachability,
        };

        Gossip::new(
            merged_members.into_iter().cloned().collect(),
            overview,
            merged_vclock,
            merged_tombstones,
        )
    }

    fn all_data_centers(&self) -> HashSet<&str> {
        self.members.iter().map(|m| m.data_center()).collect()
    }

    fn all_roles(&self) -> HashSet<&str> {
        self.members.iter().flat_map(|m| m.roles.iter()).collect()
    }

    fn is_singleton_cluster(&self) -> bool {
        self.members.len() == 1
    }

    fn is_reachable(&self, from_addr: &UniqueAddress, to_addr: &UniqueAddress) -> bool {
        if !self.has_member(to_addr) {
            false
        } else {
            self.overview.reachability.is_reachable(from_addr, to_addr)
        }
    }

    pub(crate) fn member(&self, node: &UniqueAddress) -> MaybeRef<Member> {
        self.members_map()
            .get(node)
            .map(|m| MaybeRef::Ref(*m))
            .unwrap_or_else(|| MaybeRef::Own(Member::removed(node.clone())))
    }

    pub(crate) fn has_member(&self, node: &UniqueAddress) -> bool {
        self.members_map().contains_key(node)
    }

    fn remove_all<'a>(
        &self,
        nodes: impl Iterator<Item = &'a UniqueAddress>,
        removal_timestamp: i64,
    ) -> Gossip {
        nodes.fold(self.clone(), |gossip, node| {
            gossip.remove(node, removal_timestamp)
        })
    }

    fn update(&self, updated_members: BTreeSet<Member>) -> Gossip {
        let d = self.members.difference(&updated_members).cloned().collect();
        let members = updated_members.union(&d).cloned().collect();
        let mut gossip = self.clone();
        gossip.members = members;
        gossip
    }

    fn remove(&self, node: &UniqueAddress, removal_timestamp: i64) -> Gossip {
        let mut gossip = self.clone();
        gossip.overview.seen.remove(node);
        let reachability = gossip.overview.reachability.remove(vec![node.clone()]);
        gossip.overview.reachability = reachability;
        let new_version = self.version.prune(&Node::new(Gossip::vclock_name(&node)));
        gossip.version = new_version;
        gossip.members.retain(|m| m.unique_address != *node);
        gossip.tombstones.insert(node.clone(), removal_timestamp);
        gossip
    }

    fn mark_as_down(&self, member: Member) -> Gossip {
        let mut gossip = self.clone();
        gossip.members.remove(&member);
        member.status == MemberStatus::Down;
        gossip.overview.seen.remove(&member.unique_address);
        gossip.members.insert(member);
        gossip
    }

    fn prune(&self, removed_node: &Node) -> Gossip {
        let mut gossip = self.clone();
        let new_version = self.version.prune(removed_node);
        gossip.version = new_version;
        gossip
    }

    fn prune_tombstones(&self, remove_earlier_than: i64) -> Gossip {
        let new_tombstones = self
            .tombstones
            .iter()
            .filter(|(_, timestamp)| **timestamp > remove_earlier_than)
            .map(|(n, t)| (n.clone(), *t))
            .collect::<HashMap<_, _>>();
        let mut gossip = self.clone();
        gossip.tombstones = new_tombstones;
        gossip
    }
}

impl Add<Node> for Gossip {
    type Output = Gossip;

    fn add(self, rhs: Node) -> Self::Output {
        let mut gossip = self.clone();
        gossip.version = gossip.version.add(rhs);
        gossip
    }
}

impl Add<Member> for Gossip {
    type Output = Gossip;

    fn add(self, rhs: Member) -> Self::Output {
        if self.members.contains(&rhs) {
            self.clone()
        } else {
            let mut gossip = self.clone();
            gossip.members.insert(rhs);
            gossip
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct GossipOverview {
    pub(crate) seen: HashSet<UniqueAddress>,
    pub(crate) reachability: Reachability,
}

impl GossipOverview {
    pub(crate) fn seen_digest(&self) -> String {
        let mut seen = self.seen.iter().collect_vec();
        seen.sort();
        let bytes = seen
            .into_iter()
            .map(|node| &node.address)
            .join(",")
            .into_bytes();
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        hasher
            .finalize()
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect()
    }
}

impl Display for GossipOverview {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "GossipOverview(reachability = [{}], seen = [{}])",
            self.reachability,
            self.seen.iter().join(", ")
        )
    }
}

#[derive(Debug)]
struct GossipEnvelope {
    from: UniqueAddress,
    to: UniqueAddress,
    g: Gossip,
    ser_deadline: Duration,
}

#[derive(Debug, Eq, PartialEq)]
struct GossipStatus {
    from: UniqueAddress,
    version: VectorClock,
    seen_digest: Vec<u8>,
}

impl GossipStatus {
    fn new(from: UniqueAddress, version: VectorClock, seen_digest: Vec<u8>) -> Self {
        Self {
            from,
            version,
            seen_digest,
        }
    }
}

impl Display for GossipStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "GossipStatus({},{},{})",
            self.from,
            self.version,
            self.seen_digest
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<String>()
        )
    }
}

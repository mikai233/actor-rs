use std::cmp::Ordering;
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::{Deref, DerefMut};
use std::sync::OnceLock;

use ahash::{HashMap, HashMapExt, HashSet, HashSetExt};
use anyhow::ensure;
use imstr::ImString;
use serde::{Deserialize, Serialize};

use actor_core::actor::address::Address;
use actor_core::util::version::Version;
use actor_core::{hashmap, hashset};

use crate::cluster_settings::ClusterSettings;
use crate::membership_state::MembershipState;
use crate::unique_address::UniqueAddress;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Member {
    pub unique_address: UniqueAddress,
    pub(crate) up_number: i32,
    pub status: MemberStatus,
    pub roles: HashSet<ImString>,
    pub(crate) app_version: Version,
}

impl Member {
    pub fn new(
        unique_address: UniqueAddress,
        up_number: i32,
        status: MemberStatus,
        roles: HashSet<ImString>,
        app_version: Version,
    ) -> Self {
        Self {
            unique_address,
            up_number,
            status,
            roles,
            app_version,
        }
    }

    pub fn data_center(&self) -> &str {
        self.roles
            .iter()
            .find(|r| r.starts_with(ClusterSettings::dc_role_prefix()))
            .expect("DataCenter undefined, should not be possible")
            .strip_prefix(ClusterSettings::dc_role_prefix())
            .unwrap()
    }

    pub fn has_role(&self, role: &str) -> bool {
        self.roles.contains(role)
    }

    pub fn address(&self) -> &Address {
        &self.unique_address.address
    }

    pub(crate) fn is_older_than(&self, other: &Member) -> bool {
        match Self::age_ordering(self, other) {
            Ordering::Less | Ordering::Equal => true,
            Ordering::Greater => false,
        }
    }

    fn copy(&self, status: MemberStatus) -> anyhow::Result<Member> {
        if self.status == status {
            Ok(self.clone())
        } else {
            let status_allowed = MemberStatus::allowed_transitions()
                .get(&self.status)
                .map_or(false, |allowed| allowed.contains(&status));
            ensure!(
                status_allowed,
                "Invalid member status transition [{} -> {}]",
                self.status,
                status
            );
            let member = Member::new(
                self.unique_address.clone(),
                self.up_number,
                status,
                self.roles.clone(),
                self.app_version.clone(),
            );
            Ok(member)
        }
    }

    fn copy_up(&self, up_number: i32) -> anyhow::Result<Member> {
        let member = Member::new(
            self.unique_address.clone(),
            up_number,
            self.status,
            self.roles.clone(),
            self.app_version.clone(),
        );
        member.copy(MemberStatus::Up)
    }

    pub(crate) fn pick_highest_priority<'a>(
        a: HashSet<&'a Member>,
        b: HashSet<&'a Member>,
    ) -> HashSet<&'a Member> {
        Self::pick_highest_priority_with_tombstones(a, b, &hashmap!())
    }

    pub(crate) fn pick_highest_priority_with_tombstones<'a>(
        a: HashSet<&'a Member>,
        b: HashSet<&'a Member>,
        tombstones: &HashMap<UniqueAddress, i64>,
    ) -> HashSet<&'a Member> {
        let mut grouped_by_address: HashMap<&UniqueAddress, Vec<&Member>> = HashMap::new();
        for member in a.union(&b) {
            grouped_by_address
                .entry(&member.unique_address)
                .or_insert_with(Vec::new)
                .push(&member);
        }
        grouped_by_address
            .into_iter()
            .fold(HashSet::new(), |mut acc, (_, members)| {
                if members.len() == 2 {
                    if let Some(m) = members.into_iter().reduce(Self::highest_priority_of) {
                        acc.insert(m);
                    }
                    acc
                } else {
                    if let Some(m) = members.first() {
                        if !(tombstones.contains_key(&m.unique_address)
                            || MembershipState::remove_unreachable_with_member_status()
                                .contains(&m.status))
                        {
                            acc.insert(m);
                        }
                    }
                    acc
                }
            })
    }

    fn highest_priority_of<'a>(m1: &'a Member, m2: &'a Member) -> &'a Member {
        if m1.status == m2.status {
            m1.is_older_than(m2).then(|| m2).unwrap_or(m1)
        } else {
            match (m1.status, m2.status) {
                (MemberStatus::Removed, _) => m1,
                (_, MemberStatus::Removed) => m2,
                (MemberStatus::ReadyForShutdown, _) => m1,
                (_, MemberStatus::ReadyForShutdown) => m2,
                (MemberStatus::Down, _) => m1,
                (_, MemberStatus::Down) => m2,
                (MemberStatus::Exiting, _) => m1,
                (_, MemberStatus::Exiting) => m2,
                (MemberStatus::Leaving, _) => m1,
                (_, MemberStatus::Leaving) => m2,
                (MemberStatus::Joining, _) => m2,
                (_, MemberStatus::Joining) => m1,
                (MemberStatus::WeaklyUp, _) => m2,
                (_, MemberStatus::WeaklyUp) => m1,
                (MemberStatus::PreparingForShutdown, _) => m1,
                (_, MemberStatus::PreparingForShutdown) => m2,
                (MemberStatus::Up, MemberStatus::Up) => m1,
            }
        }
    }

    pub(crate) fn removed(node: UniqueAddress) -> Member {
        let mut roles = HashSet::new();
        roles.insert(format!("{}-N/A", ClusterSettings::dc_role_prefix()).into());
        Member::new(
            node,
            i32::MAX,
            MemberStatus::Removed,
            roles,
            Version::zero().clone(),
        )
    }

    pub(crate) fn address_ordering(a: &Address, b: &Address) -> Ordering {
        a.addr.cmp(&b.addr)
    }

    pub(crate) fn leader_status_ordering(a: &Member, b: &Member) -> Ordering {
        match (a.status, b.status) {
            (a_status, b_status) if a_status == b_status => a.cmp(&b),
            (MemberStatus::Down, _) => Ordering::Greater,
            (_, MemberStatus::Down) => Ordering::Less,
            (MemberStatus::Exiting, _) => Ordering::Greater,
            (_, MemberStatus::Exiting) => Ordering::Less,
            (MemberStatus::Joining, _) => Ordering::Greater,
            (_, MemberStatus::Joining) => Ordering::Less,
            (MemberStatus::WeaklyUp, _) => Ordering::Greater,
            (_, MemberStatus::WeaklyUp) => Ordering::Less,
            _ => a.cmp(&b),
        }
    }

    pub(crate) fn ordering(a: &Member, b: &Member) -> Ordering {
        a.unique_address.cmp(&b.unique_address)
    }

    pub(crate) fn age_ordering(a: &Member, b: &Member) -> Ordering {
        if a.data_center() != b.data_center() {
            panic!("Comparing members of different data centers with isOlderThan is not allowed. [{}] vs. [{}]", a, b);
        }
        if a.up_number == b.up_number {
            Member::address_ordering(a.address(), b.address())
        } else {
            a.up_number.cmp(&b.up_number)
        }
    }
}

impl Hash for Member {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.unique_address.hash(state);
        self.status.hash(state);
        for role in &self.roles {
            role.hash(state);
        }
    }
}

impl Display for Member {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Member({}, {}{}{})",
            self.address(),
            self.status,
            if self.data_center() == ClusterSettings::default_data_center() {
                ""
            } else {
                ", "
            },
            if self.app_version == *Version::zero() {
                "".to_string()
            } else {
                self.app_version.to_string()
            }
        )
    }
}

impl PartialEq for Member {
    fn eq(&self, other: &Self) -> bool {
        self.unique_address == other.unique_address
    }
}

impl Eq for Member {}

impl PartialOrd for Member {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(Member::ordering(self, other))
    }
}

impl Ord for Member {
    fn cmp(&self, other: &Self) -> Ordering {
        Member::ordering(self, other)
    }
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum MemberStatus {
    Joining,
    WeaklyUp,
    Up,
    Leaving,
    Exiting,
    Down,
    Removed,
    PreparingForShutdown,
    ReadyForShutdown,
}

impl MemberStatus {
    fn allowed_transitions() -> &'static HashMap<MemberStatus, HashSet<MemberStatus>> {
        static ALLOWED_TRANSITION: OnceLock<HashMap<MemberStatus, HashSet<MemberStatus>>> =
            OnceLock::new();
        ALLOWED_TRANSITION.get_or_init(|| {
            let joining = hashset! {
                MemberStatus::WeaklyUp,
                MemberStatus::Up,
                MemberStatus::Leaving,
                MemberStatus::Down,
                MemberStatus::Removed,
            };
            let weakly_up = hashset! {
                MemberStatus::Up,
                MemberStatus::Leaving,
                MemberStatus::Down,
                MemberStatus::Removed,
            };
            let up = hashset! {
                MemberStatus::Leaving,
                MemberStatus::Down,
                MemberStatus::Removed,
                MemberStatus::PreparingForShutdown,
            };
            let leaving = hashset! {
                MemberStatus::Exiting,
                MemberStatus::Down,
                MemberStatus::Removed,
            };
            let down = hashset! {
                MemberStatus::Removed,
            };
            let exiting = hashset! {
                MemberStatus::Removed,
                MemberStatus::Down,
            };
            let preparing_for_shutdown = hashset! {
                MemberStatus::ReadyForShutdown,
                MemberStatus::Removed,
                MemberStatus::Leaving,
                MemberStatus::Down,
            };
            let ready_for_shutdown = hashset! {
                MemberStatus::Removed,
                MemberStatus::Leaving,
                MemberStatus::Down,
            };
            let removed = hashset! {};
            hashmap! {
                MemberStatus::Joining => joining,
                MemberStatus::WeaklyUp => weakly_up,
                MemberStatus::Up => up,
                MemberStatus::Leaving => leaving,
                MemberStatus::Exiting => exiting,
                MemberStatus::Down => down,
                MemberStatus::Removed => removed,
                MemberStatus::PreparingForShutdown => preparing_for_shutdown,
                MemberStatus::ReadyForShutdown => ready_for_shutdown,
            }
        })
    }
}

impl Display for MemberStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            MemberStatus::Joining => write!(f, "Joining"),
            MemberStatus::WeaklyUp => write!(f, "WeaklyUp"),
            MemberStatus::Up => write!(f, "Up"),
            MemberStatus::Leaving => write!(f, "Leaving"),
            MemberStatus::Exiting => write!(f, "Exiting"),
            MemberStatus::Down => write!(f, "Down"),
            MemberStatus::Removed => write!(f, "Removed"),
            MemberStatus::PreparingForShutdown => write!(f, "PreparingForShutdown"),
            MemberStatus::ReadyForShutdown => write!(f, "ReadyForShutdown"),
        }
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct MemberByAgeOrdering(pub Member);

impl Deref for MemberByAgeOrdering {
    type Target = Member;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for MemberByAgeOrdering {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<Member> for MemberByAgeOrdering {
    fn from(value: Member) -> Self {
        Self(value)
    }
}

impl Into<Member> for MemberByAgeOrdering {
    fn into(self) -> Member {
        self.0
    }
}

impl PartialOrd for MemberByAgeOrdering {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(Member::age_ordering(&self.0, &other.0))
    }
}

impl Ord for MemberByAgeOrdering {
    fn cmp(&self, other: &Self) -> Ordering {
        Member::age_ordering(&self.0, &other.0)
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct MemberByAgeOrderingRef<'a>(pub &'a Member);

impl<'a> Deref for MemberByAgeOrderingRef<'a> {
    type Target = Member;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> From<&'a Member> for MemberByAgeOrderingRef<'a> {
    fn from(value: &'a Member) -> Self {
        Self(value)
    }
}

impl<'a> Into<&'a Member> for MemberByAgeOrderingRef<'a> {
    fn into(self) -> &'a Member {
        self.0
    }
}

impl<'a> PartialOrd for MemberByAgeOrderingRef<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(Member::age_ordering(&self.0, &other.0))
    }
}

impl<'a> Ord for MemberByAgeOrderingRef<'a> {
    fn cmp(&self, other: &Self) -> Ordering {
        Member::age_ordering(&self.0, &other.0)
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct MemberByLeaderStatusOrdering(pub Member);

impl Deref for MemberByLeaderStatusOrdering {
    type Target = Member;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for MemberByLeaderStatusOrdering {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<Member> for MemberByLeaderStatusOrdering {
    fn from(value: Member) -> Self {
        Self(value)
    }
}

impl Into<Member> for MemberByLeaderStatusOrdering {
    fn into(self) -> Member {
        self.0
    }
}

impl PartialOrd for MemberByLeaderStatusOrdering {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(Member::leader_status_ordering(&self.0, &other.0))
    }
}

impl Ord for MemberByLeaderStatusOrdering {
    fn cmp(&self, other: &Self) -> Ordering {
        Member::leader_status_ordering(&self.0, &other.0)
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct MemberByLeaderStatusOrderingRef<'a>(pub &'a Member);

impl<'a> Deref for MemberByLeaderStatusOrderingRef<'a> {
    type Target = &'a Member;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> From<&'a Member> for MemberByLeaderStatusOrderingRef<'a> {
    fn from(value: &'a Member) -> Self {
        Self(value)
    }
}

impl<'a> Into<&'a Member> for MemberByLeaderStatusOrderingRef<'a> {
    fn into(self) -> &'a Member {
        self.0
    }
}

impl<'a> PartialOrd for MemberByLeaderStatusOrderingRef<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(Member::leader_status_ordering(&self.0, &other.0))
    }
}

impl<'a> Ord for MemberByLeaderStatusOrderingRef<'a> {
    fn cmp(&self, other: &Self) -> Ordering {
        Member::leader_status_ordering(&self.0, &other.0)
    }
}

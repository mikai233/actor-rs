use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};

use ahash::HashSet;
use bincode::{Decode, Encode};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use actor_core::actor::address::Address;

use crate::unique_address::UniqueAddress;

#[derive(Debug, Clone, Eq, PartialEq, Encode, Decode, Serialize, Deserialize)]
pub struct Member {
    pub addr: UniqueAddress,
    pub status: MemberStatus,
    pub roles: HashSet<String>,
    pub lease: i64,
}

impl Member {
    pub fn new(addr: UniqueAddress, status: MemberStatus, roles: HashSet<String>, lease: i64) -> Self {
        Self {
            addr,
            status,
            roles,
            lease,
        }
    }

    pub fn has_role(&self, role: &str) -> bool {
        self.roles.contains(role)
    }

    pub fn address(&self) -> &Address {
        &self.addr.address
    }
}

impl Hash for Member {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.addr.hash(state);
        self.status.hash(state);
        for role in &self.roles {
            role.hash(state);
        }
    }
}

impl Display for Member {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let roles = self.roles.iter().join(", ");
        write!(f, "Member {{addr: {}, status: {}, roles: {}, lease: {}}}", self.addr, self.status, roles, self.lease)
    }
}

#[derive(
    Debug,
    Copy,
    Clone,
    Ord,
    PartialOrd,
    Eq,
    PartialEq,
    Hash,
    Encode,
    Decode,
    Serialize,
    Deserialize
)]
pub enum MemberStatus {
    Up,
    PrepareForLeaving,
    Leaving,
    Removed,
}

impl Display for MemberStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            MemberStatus::Up => {
                write!(f, "Up")
            }
            MemberStatus::PrepareForLeaving => {
                write!(f, "PrepareForLeaving")
            }
            MemberStatus::Leaving => {
                write!(f, "Leaving")
            }
            MemberStatus::Removed => {
                write!(f, "Removed")
            }
        }
    }
}
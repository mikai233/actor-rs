use std::collections::HashSet;
use std::hash::{Hash, Hasher};

use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Encode, Decode, Serialize, Deserialize)]
pub enum MemberStatus {
    Up,
    PrepareForLeaving,
    Leaving,
    Removed,
    Down,
}
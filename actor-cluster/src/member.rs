use std::collections::HashSet;
use std::hash::{Hash, Hasher};

use bincode::{Decode, Encode};

use crate::unique_address::UniqueAddress;

#[derive(Debug, Clone, Eq, PartialEq, Encode, Decode)]
pub struct Member {
    pub unique_address: UniqueAddress,
    pub status: MemberStatus,
    pub roles: HashSet<String>,
}

impl Member {
    pub fn new(unique_address: UniqueAddress, status: MemberStatus, roles: HashSet<String>) -> Self {
        Self {
            unique_address,
            status,
            roles,
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

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Encode, Decode)]
pub enum MemberStatus {
    Joining,
    Up,
    Leaving,
    Exiting,
    Down,
    Removed,
}
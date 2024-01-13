use std::collections::HashMap;
use std::sync::RwLock;

use crate::member::Member;
use crate::unique_address::UniqueAddress;

#[derive(Debug)]
pub struct ClusterState {
    pub(crate) members: RwLock<HashMap<UniqueAddress, Member>>,
    pub(crate) self_member: RwLock<Member>,
}

impl ClusterState {
    pub fn new(member: Member) -> Self {
        Self {
            members: RwLock::new(HashMap::new()),
            self_member: RwLock::new(member),
        }
    }
}
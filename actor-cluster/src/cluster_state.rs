use ahash::{HashMap, HashMapExt};
use parking_lot::{RwLock, RwLockReadGuard};

use actor_core::actor::address::Address;

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

    pub fn is_member_up(&self, address: &Address) -> bool {
        self.members.read().iter().any(|(addr, _)| { address == &addr.address })
    }

    pub fn members(&self) -> RwLockReadGuard<HashMap<UniqueAddress, Member>> {
        self.members.read()
    }

    pub fn self_member(&self) -> RwLockReadGuard<Member> {
        self.self_member.read()
    }
}
use std::collections::HashMap;

use bincode::{Decode, Encode};

use actor_core::actor::address::Address;
use actor_derive::COrphanCodec;

use crate::member::Member;

#[derive(Debug, Clone, Encode, Decode, COrphanCodec)]
pub enum ClusterEvent {
    MemberEvent(MemberEvent),
}

#[derive(Debug, Clone, Encode, Decode)]
pub enum MemberEvent {
    MemberJoined(Member),
    MemberDowned(Member),
    MemberRemoved(Member),
    MemberExited(Member),
    MemberUp(Member),
}

#[derive(Debug, Default, Encode, Decode)]
pub struct CurrentClusterState {
    pub members: HashMap<Address, Member>,
    pub unreachable: HashMap<Address, Member>,
}
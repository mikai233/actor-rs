use std::collections::HashMap;

use bincode::{Decode, Encode};

use actor_derive::COrphanCodec;

use crate::member::Member;
use crate::unique_address::UniqueAddress;

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
    MemberLeft(Member),
    MemberUp(Member),
}

#[derive(Debug, Default, Encode, Decode)]
pub struct CurrentClusterState {
    pub members: HashMap<UniqueAddress, Member>,
    pub unreachable: HashMap<UniqueAddress, Member>,
}
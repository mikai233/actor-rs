use std::collections::HashSet;

use bincode::{Decode, Encode};

use crate::member::Member;

#[derive(Debug, Encode, Decode)]
pub enum ClusterEvent {}

#[derive(Debug, Encode, Decode)]
pub enum MemberEvent {
    MemberJoined(Member),
    MemberDowned(Member),
    MemberRemoved(Member),
    MemberExited(Member),
    MemberUp(Member),
}

#[derive(Debug, Default, Encode, Decode)]
pub struct CurrentClusterState {
    pub members: HashSet<Member>,
    pub unreachable: HashSet<Member>,
}
use std::collections::HashMap;
use std::net::SocketAddrV4;

use bincode::{Decode, Encode};
use actor_core::actor::address::Address;

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
    pub members: HashMap<Address, Member>,
    pub unreachable: HashMap<Address, Member>,
}
use std::collections::HashMap;

use bincode::{Decode, Encode};

use actor_derive::COrphanCodec;

use crate::member::Member;
use crate::unique_address::UniqueAddress;

#[derive(Debug, Clone, Encode, Decode, COrphanCodec)]
pub enum ClusterEvent {
    MemberEvent(MemberEvent),
}

impl ClusterEvent {
    pub fn member_joined(member: Member) -> Self {
        ClusterEvent::MemberEvent(MemberEvent::MemberJoined(member))
    }

    pub fn member_downed(member: Member) -> Self {
        ClusterEvent::MemberEvent(MemberEvent::MemberDowned(member))
    }

    pub fn member_removed(member: Member) -> Self {
        ClusterEvent::MemberEvent(MemberEvent::MemberRemoved(member))
    }

    pub fn member_exited(member: Member) -> Self {
        ClusterEvent::MemberEvent(MemberEvent::MemberExited(member))
    }

    pub fn member_left(member: Member) -> Self {
        ClusterEvent::MemberEvent(MemberEvent::MemberLeft(member))
    }

    pub fn member_up(member: Member) -> Self {
        ClusterEvent::MemberEvent(MemberEvent::MemberUp(member))
    }
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
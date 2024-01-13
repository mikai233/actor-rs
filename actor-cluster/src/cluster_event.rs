use std::collections::HashMap;

use bincode::{Decode, Encode};

use actor_derive::COrphanCodec;

use crate::member::{Member, MemberStatus};
use crate::unique_address::UniqueAddress;

#[derive(Debug, Clone, Encode, Decode, COrphanCodec)]
pub enum ClusterEvent {
    MemberUp(Member),
    MemberDowned(Member),
    CurrentClusterState {
        members: HashMap<UniqueAddress, Member>,
        self_member: Member,
    },
}

impl ClusterEvent {
    pub fn member_up(member: Member) -> Self {
        debug_assert!(member.status == MemberStatus::Up);
        Self::MemberUp(member)
    }

    pub fn member_downed(member: Member) -> Self {
        debug_assert!(member.status == MemberStatus::Down);
        Self::MemberDowned(member)
    }

    pub fn current_cluster_state(members: HashMap<UniqueAddress, Member>, self_member: Member) -> Self {
        Self::CurrentClusterState {
            members,
            self_member,
        }
    }
}
use ahash::HashMap;
use bincode::{Decode, Encode};

use actor_derive::COrphanCodec;

use crate::member::{Member, MemberStatus};
use crate::unique_address::UniqueAddress;

#[derive(Debug, Clone, Encode, Decode, COrphanCodec)]
pub enum ClusterEvent {
    MemberUp(Member),
    MemberPrepareForLeaving(Member),
    MemberLeaving(Member),
    MemberRemoved(Member),
    CurrentClusterState {
        members: HashMap<UniqueAddress, Member>,
        self_member: Member,
    },
    EtcdUnreachable,
}

impl ClusterEvent {
    pub fn member_up(member: Member) -> Self {
        debug_assert!(member.status == MemberStatus::Up);
        Self::MemberUp(member)
    }

    pub fn member_prepare_for_leaving(member: Member) -> Self {
        debug_assert!(member.status == MemberStatus::PrepareForLeaving);
        Self::MemberPrepareForLeaving(member)
    }

    pub fn member_leaving(member: Member) -> Self {
        debug_assert!(member.status == MemberStatus::Leaving);
        Self::MemberLeaving(member)
    }

    pub fn member_removed(member: Member) -> Self {
        debug_assert!(member.status == MemberStatus::Removed);
        Self::MemberRemoved(member)
    }

    pub fn current_cluster_state(members: HashMap<UniqueAddress, Member>, self_member: Member) -> Self {
        Self::CurrentClusterState {
            members,
            self_member,
        }
    }
}
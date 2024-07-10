use async_trait::async_trait;

use actor_cluster::cluster_event::ClusterEvent;
use actor_cluster::member::Member;
use actor_core::actor::context::ActorContext;
use actor_core::EmptyCodec;
use actor_core::Message;

use crate::shard_region::ShardRegion;

#[derive(Debug, EmptyCodec)]
pub(super) struct ClusterEventWrap(pub(super) ClusterEvent);

#[async_trait]
impl Message for ClusterEventWrap {
    type A = ShardRegion;

    async fn handle(self: Box<Self>, _context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        match self.0 {
            ClusterEvent::MemberUp(member) => {
                Self::update_member(actor, member);
            }
            ClusterEvent::MemberPrepareForLeaving(member) => {
                Self::update_member(actor, member);
            }
            ClusterEvent::MemberLeaving(member) => {
                Self::update_member(actor, member);
            }
            ClusterEvent::MemberRemoved(member) => {
                Self::remove_member(actor, member);
            }
            ClusterEvent::CurrentClusterState { members, .. } => {
                actor.members = members;
            }
            ClusterEvent::EtcdUnreachable => {}
        }
        Ok(())
    }
}

impl ClusterEventWrap {
    fn update_member(actor: &mut ShardRegion, member: Member) {
        actor.members.insert(member.unique_address.clone(), member);
    }

    fn remove_member(actor: &mut ShardRegion, member: Member) {
        actor.members.remove(&member.unique_address);
    }
}
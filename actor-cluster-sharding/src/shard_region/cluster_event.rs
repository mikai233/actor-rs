use async_trait::async_trait;

use actor_cluster::cluster_event::MemberEvent;
use actor_cluster::member::Member;
use actor_core::actor::context::ActorContext1;
use actor_core::EmptyCodec;
use actor_core::Message;

use crate::shard_region::ShardRegion;

#[derive(Debug, EmptyCodec)]
pub(super) struct ClusterEventWrap(pub(super) MemberEvent);

#[async_trait]
impl Message for ClusterEventWrap {
    type A = ShardRegion;

    async fn handle(self: Box<Self>, _context: &mut ActorContext1, actor: &mut Self::A) -> anyhow::Result<()> {
        match self.0 {
            MemberEvent::MemberUp(member) => {
                Self::update_member(actor, member);
            }
            MemberEvent::MemberPrepareForLeaving(member) => {
                Self::update_member(actor, member);
            }
            MemberEvent::MemberLeaving(member) => {
                Self::update_member(actor, member);
            }
            MemberEvent::MemberRemoved(member) => {
                Self::remove_member(actor, member);
            }
            MemberEvent::CurrentClusterState { members, .. } => {
                actor.members = members;
            }
            MemberEvent::EtcdUnreachable => {}
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
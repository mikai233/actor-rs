use async_trait::async_trait;

use actor_core::actor::context::ActorContext;
use actor_core::ext::option_ext::OptionExt;
use actor_core::Message;
use actor_derive::EmptyCodec;

use crate::cluster_event::ClusterEvent;
use crate::member::MemberStatus;
use crate::on_member_status_changed_listener::OnMemberStatusChangedListener;

#[derive(Debug, EmptyCodec)]
pub(super) struct ClusterEventWrap(pub(super) ClusterEvent);

#[async_trait]
impl Message for ClusterEventWrap {
    type A = OnMemberStatusChangedListener;

    async fn handle(self: Box<Self>, _context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        match self.0 {
            ClusterEvent::MemberUp(_) if matches!(actor.status, MemberStatus::Up) => {
                actor.callback.take().into_foreach(|callback| callback());
            }
            ClusterEvent::MemberRemoved(_) if matches!(actor.status,MemberStatus::Removed) => {
                actor.callback.take().into_foreach(|callback| callback());
            }
            _ => panic!("unreachable")
        }
        Ok(())
    }
}
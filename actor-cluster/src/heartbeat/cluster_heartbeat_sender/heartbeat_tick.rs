use async_trait::async_trait;

use actor_core::{DynMessage, Message};
use actor_core::actor::actor_selection::ActorSelectionPath;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::CEmptyCodec;

use crate::heartbeat::cluster_heartbeat_receiver::ClusterHeartbeatReceiver;
use crate::heartbeat::cluster_heartbeat_receiver::heartbeat::Heartbeat;
use crate::heartbeat::cluster_heartbeat_sender::ClusterHeartbeatSender;
use crate::member::MemberStatus;

#[derive(Debug, Clone, CEmptyCodec)]
pub(super) struct HeartbeatTick;

#[async_trait]
impl Message for HeartbeatTick {
    type A = ClusterHeartbeatSender;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        if let Some(self_member) = &actor.self_member {
            if self_member.status == MemberStatus::Up {
                for receiver in &actor.active_receivers {
                    let path = ActorSelectionPath::FullPath(ClusterHeartbeatReceiver::path(receiver.address.clone()));
                    let sel = context.actor_selection(path)?;
                    sel.tell(DynMessage::user(Heartbeat { from: self_member.unique_address.clone() }), Some(context.myself().clone()));
                }
            }
        }
        Ok(())
    }
}
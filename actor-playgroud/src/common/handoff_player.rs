use async_trait::async_trait;
use bincode::{Decode, Encode};
use tracing::info;

use actor_core::actor::context::{ActorContext1, ActorContext};
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::CMessageCodec;
use actor_core::Message;

use crate::common::player_actor::PlayerActor;

#[derive(Debug, Clone, Encode, Decode, CMessageCodec)]
pub struct HandoffPlayer;

#[async_trait]
impl Message for HandoffPlayer {
    type A = PlayerActor;

    async fn handle(self: Box<Self>, context: &mut ActorContext1, actor: &mut Self::A) -> anyhow::Result<()> {
        info!("player {} handoff", actor.id);
        context.stop(context.myself());
        Ok(())
    }
}
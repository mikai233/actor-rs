use async_trait::async_trait;
use bincode::{Decode, Encode};
use tracing::info;

use kairo_core::Message;
use kairo_core::MessageCodec;
use kairo_core::actor::context::{ActorContext, Context};

use crate::common::player_actor::PlayerActor;

#[derive(Debug, Encode, Decode, MessageCodec)]
pub struct Hello {
    pub index: i32,
    pub data: Vec<u8>,
}

#[async_trait]
impl Message for Hello {
    type A = PlayerActor;

    async fn handle(
        self: Box<Self>,
        context: &mut ActorContext,
        actor: &mut Self::A,
    ) -> anyhow::Result<()> {
        info!(
            "player {} {} receive hello {}",
            context.myself(),
            actor.id,
            self.index
        );
        Ok(())
    }
}

use async_trait::async_trait;
use bincode::{Decode, Encode};

use actor_core::actor::context::ActorContext;
use actor_core::Message;
use actor_derive::MessageCodec;

use crate::common::player_actor::PlayerActor;

#[derive(Debug, Encode, Decode, MessageCodec)]
pub struct Init;

#[async_trait]
impl Message for Init {
    type A = PlayerActor;

    async fn handle(self: Box<Self>, _context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
        Ok(())
    }
}
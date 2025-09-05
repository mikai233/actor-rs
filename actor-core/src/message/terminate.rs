use async_trait::async_trait;
use bincode::{Decode, Encode};

use actor_derive::SystemCodec;

use crate::actor::context::ActorContext;
use crate::{Actor, SystemMessage};

#[derive(Encode, Decode, SystemCodec)]
pub struct Terminate;

#[async_trait]
impl SystemMessage for Terminate {
    async fn handle(
        self: Box<Self>,
        context: &mut ActorContext,
        _actor: &mut dyn Actor,
    ) -> anyhow::Result<()> {
        context.terminate();
        Ok(())
    }
}

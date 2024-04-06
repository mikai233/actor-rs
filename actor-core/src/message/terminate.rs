use async_trait::async_trait;
use bincode::{Decode, Encode};

use actor_derive::SystemCodec;

use crate::{Actor, SystemMessage};
use crate::actor::context::ActorContext;

#[derive(Encode, Decode, SystemCodec)]
pub struct Terminate;

#[async_trait]
impl SystemMessage for Terminate {
    async fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut dyn Actor) -> eyre::Result<()> {
        context.terminate();
        Ok(())
    }
}
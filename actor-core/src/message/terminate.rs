use async_trait::async_trait;
use bincode::{Decode, Encode};

use actor_derive::SystemMessageCodec;

use crate::{Actor, SystemMessage};
use crate::actor::context::ActorContext;

#[derive(Encode, Decode, SystemMessageCodec)]
pub struct Terminate;

#[async_trait]
impl SystemMessage for Terminate {
    async fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut dyn Actor) -> anyhow::Result<()> {
        context.terminate();
        Ok(())
    }
}
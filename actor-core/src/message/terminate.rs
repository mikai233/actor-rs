use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use actor_derive::SystemMessageCodec;

use crate::{Actor, SystemMessage};
use crate::actor::context::ActorContext;

#[derive(Serialize, Deserialize, SystemMessageCodec)]
pub struct Terminate;

#[async_trait]
impl SystemMessage for Terminate {
    async fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut dyn Actor) -> anyhow::Result<()> {
        context.terminate();
        Ok(())
    }
}
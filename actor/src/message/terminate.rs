use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use actor_derive::SystemMessageCodec;

use crate::{ SystemMessage};
use crate::context::ActorContext;

#[derive(Serialize, Deserialize, SystemMessageCodec)]
pub(crate) struct Terminate;

#[async_trait]
impl SystemMessage for Terminate {
    async fn handle(self: Box<Self>, context: &mut ActorContext) -> anyhow::Result<()> {
        context.terminate();
        Ok(())
    }
}
use actor_derive::Message;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::actor::context::Context;

#[derive(Debug, Copy, Clone, Message, Serialize, Deserialize, derive_more::Display)]
#[display("Terminate")]
pub struct Terminate;

#[async_trait]
impl SystemMessage for Terminate {
    async fn handle(self: Box<Self>, context: &mut Context, _actor: &mut dyn Actor) -> anyhow::Result<()> {
        context.terminate();
        Ok(())
    }
}
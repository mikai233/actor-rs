use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::trace;

use actor_derive::SystemMessageCodec;

use crate::{Actor, SystemMessage};
use crate::actor::context::ActorContext;
use crate::actor::state::ActorState;

#[derive(Debug, Serialize, Deserialize, SystemMessageCodec)]
pub struct Recreate {
    pub error: Option<String>,
}

#[async_trait]
impl SystemMessage for Recreate {
    async fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut dyn Actor) -> anyhow::Result<()> {
        context.state = ActorState::Recreate;
        trace!("{} recreate", context.myself);
        Ok(())
    }
}
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::trace;

use actor_derive::Message;

use crate::actor::context::ActorContext1;
use crate::actor::state::ActorState;

#[derive(Debug, Copy, Clone, Message, Serialize, Deserialize, derive_more::Display)]
#[cloneable]
#[display("Suspend")]
pub struct Suspend;

#[async_trait]
impl SystemMessage for Suspend {
    async fn handle(self: Box<Self>, context: &mut ActorContext1, _actor: &mut dyn Actor) -> anyhow::Result<()> {
        context.state = ActorState::Suspend;
        trace!("{} suspend", context.myself);
        Ok(())
    }
}
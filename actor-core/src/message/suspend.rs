use async_trait::async_trait;
use bincode::{Decode, Encode};
use tracing::trace;

use actor_derive::SystemCodec;

use crate::{Actor, SystemMessage};
use crate::actor::context::ActorContext;
use crate::actor::state::ActorState;

#[derive(Debug, Encode, Decode, SystemCodec)]
pub struct Suspend;

#[async_trait]
impl SystemMessage for Suspend {
    async fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut dyn Actor) -> eyre::Result<()> {
        context.state = ActorState::Suspend;
        trace!("{} suspend", context.myself);
        Ok(())
    }
}
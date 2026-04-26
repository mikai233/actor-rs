use async_trait::async_trait;
use bincode::{Decode, Encode};
use tracing::trace;

use kairo_derive::SystemCodec;

use crate::actor::context::ActorContext;
use crate::actor::state::ActorState;
use crate::{Actor, SystemMessage};

#[derive(Debug, Encode, Decode, SystemCodec)]
pub struct Suspend;

#[async_trait]
impl SystemMessage for Suspend {
    async fn handle(
        self: Box<Self>,
        context: &mut ActorContext,
        _actor: &mut dyn Actor,
    ) -> anyhow::Result<()> {
        context.state = ActorState::Suspend;
        trace!("{} suspend", context.myself);
        Ok(())
    }
}

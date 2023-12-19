use async_trait::async_trait;
use bincode::{Decode, Encode};
use tracing::trace;

use actor_derive::SystemMessageCodec;

use crate::{Actor, SystemMessage};
use crate::actor::context::ActorContext;
use crate::actor::state::ActorState;

#[derive(Debug, Encode, Decode, SystemMessageCodec)]
pub struct Resume;

#[async_trait]
impl SystemMessage for Resume {
    async fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut dyn Actor) -> anyhow::Result<()> {
        context.state = ActorState::Started;
        trace!("{} resume", context.myself);
        Ok(())
    }
}
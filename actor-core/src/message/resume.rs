use async_trait::async_trait;
use bincode::{Decode, Encode};
use tracing::trace;

use actor_derive::SystemCodec;

use crate::{Actor, SystemMessage};
use crate::actor::context::{ActorContext, Context};
use crate::actor::state::ActorState;

#[derive(Debug, Encode, Decode, SystemCodec)]
pub struct Resume;

#[async_trait]
impl SystemMessage for Resume {
    async fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut dyn Actor) -> eyre::Result<()> {
        debug_assert!(matches!(context.state, ActorState::Suspend));
        context.state = ActorState::Started;
        for child in context.children() {
            child.resume();
        }
        trace!("{} resume", context.myself);
        Ok(())
    }
}
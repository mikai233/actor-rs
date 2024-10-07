use crate::actor::context::{ActorContext1, ActorContext};
use crate::actor::state::ActorState;
use actor_derive::Message;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::trace;

#[derive(Debug, Copy, Clone, Message, Serialize, Deserialize, derive_more::Display)]
#[cloneable]
#[display("Resume")]
pub struct Resume;

#[async_trait]
impl SystemMessage for Resume {
    async fn handle(self: Box<Self>, context: &mut ActorContext1, _actor: &mut dyn Actor) -> anyhow::Result<()> {
        debug_assert!(matches!(context.state, ActorState::Suspend));
        context.state = ActorState::Started;
        for child in context.children() {
            child.resume();
        }
        trace!("{} resume", context.myself);
        Ok(())
    }
}
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use actor_derive::SystemMessageCodec;

use crate::actor::context::ActorContext;
use crate::actor::state::ActorState;
use crate::{Actor, SystemMessage};

#[derive(Debug, Serialize, Deserialize, SystemMessageCodec)]
pub struct Suspend;

#[async_trait]
impl SystemMessage for Suspend {
    async fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut dyn Actor) -> anyhow::Result<()> {
        context.state = ActorState::Suspend;
        Ok(())
    }
}
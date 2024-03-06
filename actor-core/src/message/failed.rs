use async_trait::async_trait;
use bincode::{Decode, Encode};

use actor_derive::SystemCodec;

use crate::{Actor, SystemMessage};
use crate::actor::context::ActorContext;
use crate::actor_ref::ActorRef;

#[derive(Debug, Encode, Decode, SystemCodec)]
pub struct Failed {
    pub child: ActorRef,
}

#[async_trait]
impl SystemMessage for Failed {
    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut dyn Actor) -> anyhow::Result<()> {
        let Self { child } = *self;
        actor.supervisor_strategy().handle_failure(context, &child);
        Ok(())
    }
}
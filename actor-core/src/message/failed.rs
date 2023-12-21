use async_trait::async_trait;
use bincode::{Decode, Encode};

use actor_derive::SystemMessageCodec;

use crate::{Actor, SystemMessage};
use crate::actor::actor_ref::ActorRef;
use crate::actor::context::ActorContext;

#[derive(Debug, Encode, Decode, SystemMessageCodec)]
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
use async_trait::async_trait;
use bincode::{Decode, Encode};

use actor_derive::SystemCodec;

use crate::{Actor, SystemMessage};
use crate::actor::context::ActorContext;
use crate::actor_ref::ActorRef;

#[derive(Debug, Clone, Encode, Decode, SystemCodec)]
pub struct DeathWatchNotification {
    pub actor: ActorRef,
    pub existence_confirmed: bool,
    pub address_terminated: bool,
}

#[async_trait]
impl SystemMessage for DeathWatchNotification {
    async fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut dyn Actor) -> anyhow::Result<()> {
        context.watched_actor_terminated(self.actor);
        Ok(())
    }
}
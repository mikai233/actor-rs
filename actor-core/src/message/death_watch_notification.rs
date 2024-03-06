use async_trait::async_trait;
use bincode::{Decode, Encode};

use actor_derive::SystemCodec;

use crate::{Actor, SystemMessage};
use crate::actor::context::ActorContext;
use crate::actor_ref::ActorRef;

#[derive(Debug, Encode, Decode, SystemCodec)]
pub struct DeathWatchNotification(pub(crate) ActorRef);

#[async_trait]
impl SystemMessage for DeathWatchNotification {
    async fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut dyn Actor) -> anyhow::Result<()> {
        context.watched_actor_terminated(self.0);
        Ok(())
    }
}
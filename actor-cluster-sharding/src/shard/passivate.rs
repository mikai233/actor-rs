use async_trait::async_trait;
use tracing::warn;

use actor_core::{DynMessage, Message};
use actor_core::actor::context::{ActorContext, Context};
use actor_derive::EmptyCodec;

use crate::shard::Shard;

#[derive(Debug, EmptyCodec)]
pub struct Passivate {
    pub stop_message: DynMessage,
}

#[async_trait]
impl Message for Passivate {
    type A = Shard;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> eyre::Result<()> {
        match context.sender() {
            None => {
                let name = self.stop_message.name();
                let type_name = &actor.type_name;
                warn!("Ignore Passivate:{} message for {} because Passivate sender is none", name, type_name);
            }
            Some(entity) => {
                actor.passivate(entity, self.stop_message)?;
            }
        }
        Ok(())
    }
}
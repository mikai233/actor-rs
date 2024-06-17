use async_trait::async_trait;
use tracing::warn;

use actor_core::{CodecMessage, DynMessage, Message};
use actor_core::actor::context::{ActorContext, Context};
use actor_core::EmptyCodec;

use crate::shard::Shard;

#[derive(Debug, EmptyCodec)]
pub struct Passivate {
    pub stop_message: DynMessage,
}

impl Passivate {
    pub fn new<M>(stop_message: M) -> Self
    where
        M: CodecMessage,
    {
        Self { stop_message: stop_message.into_dyn() }
    }
}

#[async_trait]
impl Message for Passivate {
    type A = Shard;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
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
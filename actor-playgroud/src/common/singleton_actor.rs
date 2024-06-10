use async_trait::async_trait;

use actor_core::{Actor, DynMessage};
use actor_core::actor::context::ActorContext;

#[derive(Debug)]
pub struct SingletonActor;

#[async_trait]
impl Actor for SingletonActor {
    async fn on_recv(&mut self, context: &mut ActorContext, message: DynMessage) -> anyhow::Result<()> {
        Self::handle_message(self, context, message).await
    }
}
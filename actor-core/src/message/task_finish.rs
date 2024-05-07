use async_trait::async_trait;
use tracing::{debug, error};

use actor_derive::SystemEmptyCodec;

use crate::{Actor, SystemMessage};
use crate::actor::context::ActorContext;

#[derive(Debug, SystemEmptyCodec)]
pub(crate) struct TaskFinish {
    pub(crate) name: String,
}

#[async_trait]
impl SystemMessage for TaskFinish {
    async fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut dyn Actor) -> eyre::Result<()> {
        match context.abort_handles.remove(&self.name) {
            None => {
                error!("finish task not found: {}", self.name);
            }
            Some(_) => {
                debug!("finish task: {}", self.name);
            }
        }
        Ok(())
    }
}
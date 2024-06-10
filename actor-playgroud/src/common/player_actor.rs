use std::time::SystemTime;

use async_trait::async_trait;
use tracing::info;

use actor_cluster_sharding::shard_region::ImShardId;
use actor_core::{Actor, DynMessage};
use actor_core::actor::context::ActorContext;
use actor_core::actor::props::PropsBuilder;

#[derive(Debug)]
pub struct PlayerActor {
    pub id: ImShardId,
    pub count: usize,
    pub start: Option<SystemTime>,
}

#[async_trait]
impl Actor for PlayerActor {
    async fn started(&mut self, _context: &mut ActorContext) -> anyhow::Result<()> {
        info!("player {} started", self.id);
        Ok(())
    }

    async fn on_recv(&mut self, context: &mut ActorContext, message: DynMessage) -> anyhow::Result<()> {
        Self::handle_message(self, context, message).await
    }
}

pub fn player_actor_builder() -> PropsBuilder<ImShardId> {
    PropsBuilder::new(|id| {
        let player = PlayerActor { id, count: 0, start: None };
        Ok(player)
    })
}
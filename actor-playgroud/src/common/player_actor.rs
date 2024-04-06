use std::time::SystemTime;

use async_trait::async_trait;
use tracing::info;

use actor_cluster_sharding::shard_region::ImShardId;
use actor_core::Actor;
use actor_core::actor::context::ActorContext;

#[derive(Debug)]
pub struct PlayerActor {
    pub id: ImShardId,
    pub count: usize,
    pub start: Option<SystemTime>,
}

#[async_trait]
impl Actor for PlayerActor {
    async fn started(&mut self, _context: &mut ActorContext) -> eyre::Result<()> {
        info!("player {} started", self.id);
        Ok(())
    }
}
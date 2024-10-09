use std::ops::Not;

use async_trait::async_trait;
use bincode::{Decode, Encode};
use tracing::debug;

use actor_core::actor::context::Context;
use actor_core::actor_ref::ActorRef;
use actor_core::CMessageCodec;
use actor_core::Message;

use crate::shard_coordinator::ShardCoordinator;

#[derive(Debug, Clone, Encode, Decode, CMessageCodec)]
pub(crate) struct GracefulShutdownReq {
    pub(crate) shard_region: ActorRef,
}

#[async_trait]
impl Message for GracefulShutdownReq {
    type A = ShardCoordinator;

    async fn handle(self: Box<Self>, context: &mut Context, actor: &mut Self::A) -> anyhow::Result<()> {
        let region = self.shard_region;
        if actor.graceful_shutdown_in_progress.contains(&region).not() {
            match actor.state.regions.get(&region) {
                None => {
                    debug!("{}: Unknown region requested graceful shutdown [{}]", actor.type_name, region);
                }
                Some(shards) => {
                    debug!("{}: Graceful shutdown of region [{}] with [{}] shards", actor.type_name, region, shards.len());
                    actor.graceful_shutdown_in_progress.insert(region.clone());
                    actor.shutdown_shards(context, region, shards.clone())?;
                }
            }
        }
        Ok(())
    }
}
use std::ops::Not;

use async_trait::async_trait;
use itertools::Itertools;
use tracing::info;

use actor_core::actor::context::ActorContext;
use actor_core::Message;
use actor_derive::EmptyCodec;

use crate::shard_coordinator::ShardCoordinator;

#[derive(Debug, EmptyCodec)]
pub(super) struct StopShardTimeout(pub(super) uuid::Uuid);

#[async_trait]
impl Message for StopShardTimeout {
    type A = ShardCoordinator;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let request_id = self.0;
        let mut timed_out_shards = vec![];
        let mut empty_shards = vec![];
        //TODO 更好的实现方式
        for (shard_id, waiting) in &mut actor.waiting_for_shards_to_stop {
            waiting.retain(|(_, id)| {
                let timed_out = &request_id == id;
                if timed_out {
                    timed_out_shards.push(shard_id);
                }
                !timed_out
            });
            if waiting.is_empty() {
                empty_shards.push(shard_id.clone());
            }
        }
        if timed_out_shards.is_empty().not() {
            let timed_out_shards_str = timed_out_shards.iter().join(", ");
            info!("{}: Stop shard request [{}] timed out for shards [{}]", actor.type_name, request_id, timed_out_shards_str);
        }
        for shard in empty_shards {
            actor.waiting_for_shards_to_stop.remove(&shard);
        }
        Ok(())
    }
}
use std::str::FromStr;

use actor_cluster_sharding::message_extractor::MessageExtractor;
use actor_cluster_sharding::shard_region::{EntityId, ShardId};
use actor_cluster_sharding::ShardEnvelope;

pub const SHARD_MOD: usize = 10000;

#[derive(Debug, Clone)]
pub struct PlayerMessageExtractor;

impl MessageExtractor for PlayerMessageExtractor {
    fn entity_id(&self, message: &ShardEnvelope) -> EntityId {
        message.entity_id.clone()
    }

    fn shard_id(&self, message: &ShardEnvelope) -> ShardId {
        let entity_id = usize::from_str(&message.entity_id).unwrap();
        let shard = entity_id % SHARD_MOD;
        shard.to_string()
    }
}
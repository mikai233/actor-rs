use ahash::HashMap;
use async_trait::async_trait;
use bincode::{Decode, Encode};
use itertools::Itertools;
use tracing::{debug, error};

use actor_core::actor::context::Context;
use actor_core::actor_ref::ActorRef;
use actor_core::Message;
use actor_core::MessageCodec;

use crate::shard_region::{ShardId, ShardRegion};

#[derive(Debug, Encode, Decode, MessageCodec)]
pub(crate) struct ShardHomes {
    pub(crate) homes: HashMap<ActorRef, Vec<ShardId>>,
}

#[async_trait]
impl Message for ShardHomes {
    type A = ShardRegion;

    async fn handle(self: Box<Self>, context: &mut Context, actor: &mut Self::A) -> anyhow::Result<()> {
        let homes_str = self.homes.keys().join(", ");
        debug!("Got shard homes for regions [{homes_str}]");
        for (shard_region_ref, shards) in self.homes {
            for shard_id in shards {
                if let Some(error) = actor.receive_shard_home(context, shard_id.into(), shard_region_ref.clone()).err() {
                    error!("{:?}", error);
                }
            }
        }
        Ok(())
    }
}
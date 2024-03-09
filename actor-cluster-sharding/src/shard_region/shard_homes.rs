use std::collections::HashMap;

use async_trait::async_trait;
use bincode::{Decode, Encode};
use tracing::{debug, error};

use actor_core::actor::context::ActorContext;
use actor_core::actor_ref::ActorRef;
use actor_core::Message;
use actor_derive::MessageCodec;

use crate::shard_region::{ShardId, ShardRegion};

#[derive(Debug, Encode, Decode, MessageCodec)]
pub(crate) struct ShardHomes {
    pub(crate) homes: HashMap<ActorRef, Vec<ShardId>>,
}

#[async_trait]
impl Message for ShardHomes {
    type A = ShardRegion;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let homes_str = self.homes.keys().map(ToString::to_string).collect::<Vec<_>>().join(", ");
        debug!("Got shard homes for regions [{homes_str}]");
        for (shard_region_ref, shards) in self.homes {
            for shard_id in shards {
                if let Some(error) = actor.receive_shard_home(context, shard_id, shard_region_ref.clone()).err() {
                    error!("{:#?}", error);
                }
            }
        }
        Ok(())
    }
}
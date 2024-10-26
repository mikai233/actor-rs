use crate::shard_region::{ShardId, ShardRegion};
use actor_core::actor::behavior::Behavior;
use actor_core::actor::context::Context;
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::actor_ref::ActorRef;
use actor_core::message::handler::MessageHandler;
use actor_core::Message;
use actor_core::MessageCodec;
use ahash::HashMap;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use tracing::{debug, error};

#[derive(Debug, Serialize, Deserialize, Message, MessageCodec, derive_more::Constructor)]
pub(crate) struct ShardHomes {
    pub(crate) homes: HashMap<ActorRef, Vec<ShardId>>,
}

impl Display for ShardHomes {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut homes = self
            .homes
            .iter()
            .map(|(k, v)| format!("{}: [{}]", k, v.iter().join(", ")));
        write!(f, "ShardHomes {{ homes: {} }}", homes.join(", "))
    }
}

impl MessageHandler<ShardRegion> for ShardHomes {
    fn handle(
        actor: &mut ShardRegion,
        ctx: &mut <ShardRegion as Actor>::Context,
        message: Self,
        _: Option<ActorRef>,
        _: &Receive<ShardRegion>,
    ) -> anyhow::Result<Behavior<ShardRegion>> {
        let home_regions = message.homes.keys().join(", ");
        debug!("Got shard homes for regions [{home_regions}]");
        for (shard_region_ref, shards) in message.homes {
            for shard_id in shards {
                if let Some(error) = actor
                    .receive_shard_home(ctx, shard_id.into(), shard_region_ref.clone())
                    .err()
                {
                    error!("{:?}", error);
                }
            }
        }
        Ok(Behavior::same())
    }
}

use crate::shard_coordinator::ShardCoordinator;
use crate::shard_region::ImShardId;
use actor_core::actor::behavior::Behavior;
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::actor_ref::ActorRef;
use actor_core::message::handler::MessageHandler;
use actor_core::Message;
use ahash::HashSet;
use itertools::Itertools;
use std::fmt::{Display, Formatter};

#[derive(Debug, Message)]
pub(super) struct RebalanceResult {
    pub(super) shards: HashSet<ImShardId>,
}

impl Display for RebalanceResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RebalanceResult {{ shards: {} }}",
            self.shards.iter().join(", ")
        )
    }
}

impl MessageHandler<ShardCoordinator> for RebalanceResult {
    fn handle(
        actor: &mut ShardCoordinator,
        ctx: &mut <ShardCoordinator as Actor>::Context,
        message: Self,
        _: Option<ActorRef>,
        _: &Receive<ShardCoordinator>,
    ) -> anyhow::Result<Behavior<ShardCoordinator>> {
        actor.continue_rebalance(ctx, message.shards)?;
        Ok(Behavior::same())
    }
}

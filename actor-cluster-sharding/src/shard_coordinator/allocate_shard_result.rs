use crate::shard_coordinator::ShardCoordinator;
use crate::shard_region::ImShardId;
use actor_core::actor::behavior::Behavior;
use actor_core::actor::context::Context;
use actor_core::actor::receive::Receive;
use actor_core::actor_ref::ActorRef;
use actor_core::message::handler::MessageHandler;
use actor_core::Message;
use std::fmt::{Display, Formatter};
use tracing::debug;

#[derive(Debug, Message)]
pub(super) struct AllocateShardResult {
    pub(super) shard: ImShardId,
    pub(super) shard_region: Option<ActorRef>,
    pub(super) get_shard_home_sender: ActorRef,
}

impl Display for AllocateShardResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "AllocateShardResult {{ shard: {}, shard_region: {:?}, get_shard_home_sender: {} }}",
            self.shard, self.shard_region, self.get_shard_home_sender,
        )
    }
}

impl MessageHandler<ShardCoordinator> for AllocateShardResult {
    fn handle(
        actor: &mut ShardCoordinator,
        ctx: &mut ShardCoordinator::Context,
        message: Self,
        sender: Option<ActorRef>,
        _: &Receive<ShardCoordinator>,
    ) -> anyhow::Result<Behavior<ShardCoordinator>> {
        let Self {
            shard,
            shard_region,
            get_shard_home_sender,
        } = message;
        match shard_region {
            None => {
                debug!(
                    "{}: Shard [{}] allocation failed. It will be retried.",
                    actor.type_name, shard
                )
            }
            Some(shard_region) => {
                actor
                    .continue_get_shard_home(ctx, shard, shard_region, get_shard_home_sender)
                    .await;
            }
        }
        Ok(Behavior::same())
    }
}

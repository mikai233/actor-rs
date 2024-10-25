use std::fmt::{Display, Formatter};
use std::ops::Not;

use crate::shard_coordinator::ShardCoordinator;
use actor_core::actor::behavior::Behavior;
use actor_core::actor::context::Context;
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::actor_ref::ActorRef;
use actor_core::message::handler::MessageHandler;
use actor_core::Message;
use itertools::Itertools;
use tracing::info;

#[derive(Debug, Message)]
pub(super) struct StopShardTimeout(pub(super) uuid::Uuid);

impl Display for StopShardTimeout {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "StopShardTimeout({})", self.0)
    }
}

impl MessageHandler<ShardCoordinator> for StopShardTimeout {
    fn handle(
        actor: &mut ShardCoordinator,
        ctx: &mut <ShardCoordinator as Actor>::Context,
        message: Self,
        sender: Option<ActorRef>,
        _: &Receive<ShardCoordinator>,
    ) -> anyhow::Result<Behavior<ShardCoordinator>> {
        let request_id = message.0;
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
            info!(
                "{}: Stop shard request [{}] timed out for shards [{}]",
                actor.type_name, request_id, timed_out_shards_str
            );
        }
        for shard in empty_shards {
            actor.waiting_for_shards_to_stop.remove(&shard);
        }
        Ok(Behavior::same())
    }
}

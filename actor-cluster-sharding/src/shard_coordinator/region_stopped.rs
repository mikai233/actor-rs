use crate::shard_coordinator::ShardCoordinator;
use actor_core::actor::behavior::Behavior;
use actor_core::actor::context::Context;
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::actor_ref::ActorRef;
use actor_core::message::handler::MessageHandler;
use actor_core::Message;
use actor_core::MessageCodec;
use serde::{Deserialize, Serialize};
use tracing::debug;

#[derive(
    Debug,
    Serialize,
    Deserialize,
    Message,
    MessageCodec,
    derive_more::Display,
    derive_more::Constructor,
)]
#[display("RegionStopped {{ shard_region: {shard_region} }}")]
pub(crate) struct RegionStopped {
    pub(crate) shard_region: ActorRef,
}

impl MessageHandler<ShardCoordinator> for RegionStopped {
    fn handle(
        actor: &mut ShardCoordinator,
        ctx: &mut <ShardCoordinator as Actor>::Context,
        message: Self,
        sender: Option<ActorRef>,
        _: &Receive<ShardCoordinator>,
    ) -> anyhow::Result<Behavior<ShardCoordinator>> {
        let shard_region = message.shard_region;
        debug!(
            "{}: ShardRegion stopped: [{}]",
            actor.type_name, shard_region
        );
        actor.region_terminated(ctx, shard_region).await;
        Ok(Behavior::same())
    }
}

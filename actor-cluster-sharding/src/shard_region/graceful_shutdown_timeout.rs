use actor_core::actor::behavior::Behavior;
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::actor_ref::ActorRef;
use actor_core::message::handler::MessageHandler;
use actor_core::Message;
use itertools::Itertools;
use tracing::warn;

use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;

use crate::shard_region::ShardRegion;

#[derive(Debug, Message, derive_more::Display)]
#[display("GracefulShutdownTimeout")]
pub(super) struct GracefulShutdownTimeout;

impl MessageHandler<ShardRegion> for GracefulShutdownTimeout {
    fn handle(
        actor: &mut ShardRegion,
        ctx: &mut <ShardRegion as Actor>::Context,
        message: Self,
        sender: Option<ActorRef>,
        _: &Receive<ShardRegion>,
    ) -> anyhow::Result<Behavior<ShardRegion>> {
        let shards = actor.shards.keys().join(", ");
        let buffer_size = actor.shard_buffers.total_size();
        warn!(
            "{}: Graceful shutdown of shard region {} timed out, region will be stopped. Remaining shards [{}], remaining buffered messages [{}]",
            actor.type_name,
            ctx.myself(),
            shards,
            buffer_size,
        );
        ctx.stop(ctx.myself());
        Ok(Behavior::same())
    }
}

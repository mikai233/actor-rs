use crate::shard_region::{ShardId, ShardRegion};
use actor_core::actor::behavior::Behavior;
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::actor_ref::ActorRef;
use actor_core::message::handler::MessageHandler;
use actor_core::Message;
use actor_core::MessageCodec;
use serde::{Deserialize, Serialize};

#[derive(
    Debug,
    Serialize,
    Deserialize,
    Message,
    MessageCodec,
    derive_more::Display,
    derive_more::Constructor,
)]
#[display("ShardHome {{ shard: {shard}, shard_region: {shard_region} }}")]
pub(crate) struct ShardHome {
    pub(crate) shard: ShardId,
    pub(crate) shard_region: ActorRef,
}

impl MessageHandler<ShardRegion> for ShardHome {
    fn handle(
        actor: &mut ShardRegion,
        ctx: &mut <ShardRegion as Actor>::Context,
        message: Self,
        _: Option<ActorRef>,
        _: &Receive<ShardRegion>,
    ) -> anyhow::Result<Behavior<ShardRegion>> {
        actor.receive_shard_home(ctx, message.shard.into(), message.shard_region)?;
        Ok(Behavior::same())
    }
}

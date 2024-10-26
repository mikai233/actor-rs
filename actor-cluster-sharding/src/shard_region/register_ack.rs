use std::ops::Not;

use crate::shard_region::ShardRegion;
use actor_core::actor::behavior::Behavior;
use actor_core::actor::context::ActorContext;
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::actor_ref::ActorRef;
use actor_core::message::handler::MessageHandler;
use actor_core::{Message, MessageCodec};

#[derive(Debug, Message, MessageCodec, derive_more::Display, derive_more::Constructor)]
#[display("RegisterAck {{ coordinator: {coordinator} }}")]
pub(crate) struct RegisterAck {
    pub(crate) coordinator: ActorRef,
}

impl MessageHandler<ShardRegion> for RegisterAck {
    fn handle(
        actor: &mut ShardRegion,
        ctx: &mut <ShardRegion as Actor>::Context,
        message: Self,
        _: Option<ActorRef>,
        _: &Receive<ShardRegion>,
    ) -> anyhow::Result<Behavior<ShardRegion>> {
        if ctx.is_watching(&message.coordinator).not() {
            ctx.watch(&message.coordinator)?;
        }
        actor.coordinator = Some(message.coordinator);
        actor.finish_registration();
        actor.try_request_shard_buffer_homes(ctx);
        Ok(Behavior::same())
    }
}

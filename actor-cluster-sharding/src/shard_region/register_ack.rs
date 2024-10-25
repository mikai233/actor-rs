use std::ops::Not;

use crate::shard_region::coordinator_terminated::CoordinatorTerminated;
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
        sender: Option<ActorRef>,
        _: &Receive<ShardRegion>,
    ) -> anyhow::Result<Behavior<ShardRegion>> {
        if ctx.is_watching(&actor.coordinator).not() {
            context.watch_with(self.coordinator.clone(), CoordinatorTerminated::new)?;
        }
        actor.coordinator = Some(ctx.coordinator);
        actor.finish_registration();
        actor.try_request_shard_buffer_homes(ctx);
        Ok(Behavior::same())
    }
}

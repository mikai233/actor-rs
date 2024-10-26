use actor_core::actor::behavior::Behavior;
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::actor_ref::ActorRef;
use actor_core::message::handler::MessageHandler;

use crate::message_extractor::ShardEnvelope;
use crate::shard::Shard;
use actor_core::actor::context::ActorContext;

impl MessageHandler<Shard> for ShardEnvelope {
    fn handle(
        actor: &mut Shard,
        ctx: &mut <Shard as Actor>::Context,
        message: Self,
        sender: Option<ActorRef>,
        _: &Receive<Shard>,
    ) -> anyhow::Result<Behavior<Shard>> {
        actor.deliver_message(ctx, message, sender)?;
        Ok(Behavior::same())
    }
}

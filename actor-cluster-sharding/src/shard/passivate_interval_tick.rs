use actor_core::actor::behavior::Behavior;
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::actor_ref::ActorRef;
use actor_core::message::handler::MessageHandler;

use actor_core::actor::context::Context;
use actor_core::Message;

use crate::shard::Shard;

#[derive(Debug, Message, derive_more::Display)]
#[display("PassivateIntervalTick")]
struct PassivateIntervalTick;

impl MessageHandler<Shard> for PassivateIntervalTick {
    fn handle(
        actor: &mut Shard,
        ctx: &mut <Shard as Actor>::Context,
        message: Self,
        sender: Option<ActorRef>,
        _: &Receive<Shard>,
    ) -> anyhow::Result<Behavior<Shard>> {
        todo!()
    }
}

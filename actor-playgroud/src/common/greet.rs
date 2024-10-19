use crate::common::singleton_actor::SingletonActor;
use actor_core::actor::behavior::Behavior;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::actor_ref::ActorRef;
use actor_core::message::handler::MessageHandler;
use actor_core::Message;
use actor_core::MessageCodec;
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Debug, Serialize, Deserialize, Message, MessageCodec)]
pub struct Greet(pub usize);

impl MessageHandler<SingletonActor> for Greet {
    fn handle(
        _: &mut SingletonActor,
        ctx: &mut <SingletonActor as Actor>::Context,
        message: Self,
        _: Option<ActorRef>,
        _: &Receive<SingletonActor>,
    ) -> anyhow::Result<Behavior<SingletonActor>> {
        info!("{} recv {}", ctx.myself(), message);
        Ok(Behavior::same())
    }
}

use actor_core::actor::behavior::Behavior;
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::actor_ref::ActorRef;
use actor_core::message::handler::MessageHandler;
use actor_core::message::{DynMessage, Message};
use tracing::warn;

use crate::shard::Shard;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::Message;

#[derive(Debug, Message, derive_more::Display)]
#[display("Passivate {{ stop_message: {stop_message} }}")]
pub struct Passivate {
    pub stop_message: DynMessage,
}

impl Passivate {
    pub fn new<M>(stop_message: M) -> Self
    where
        M: Message,
    {
        Self {
            stop_message: Box::new(stop_message),
        }
    }
}

impl MessageHandler<Shard> for Passivate {
    fn handle(
        actor: &mut Shard,
        ctx: &mut <Shard as Actor>::Context,
        message: Self,
        sender: Option<ActorRef>,
        _: &Receive<Shard>,
    ) -> anyhow::Result<Behavior<Shard>> {
        match sender {
            None => {
                let type_name = &actor.type_name;
                warn!(
                    "Ignore Passivate:{} message for {} because Passivate Sender is None",
                    message, type_name
                );
            }
            Some(entity) => {
                actor.passivate(&entity, message.stop_message)?;
            }
        }
        Ok(Behavior::same())
    }
}

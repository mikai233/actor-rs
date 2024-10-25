use crate::shard_coordinator::ShardCoordinator;
use actor_core::actor::behavior::Behavior;
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::actor_ref::ActorRef;
use actor_core::message::handler::MessageHandler;
use actor_core::{Message, MessageCodec};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Message, MessageCodec, derive_more::Display)]
#[cloneable]
#[display("TerminateCoordinator")]
pub(crate) struct TerminateCoordinator;

impl MessageHandler<ShardCoordinator> for TerminateCoordinator {
    fn handle(
        actor: &mut ShardCoordinator,
        ctx: &mut <ShardCoordinator as Actor>::Context,
        message: Self,
        sender: Option<ActorRef>,
        _: &Receive<ShardCoordinator>,
    ) -> anyhow::Result<Behavior<ShardCoordinator>> {
        actor.terminate(ctx);
        todo!()
    }
}

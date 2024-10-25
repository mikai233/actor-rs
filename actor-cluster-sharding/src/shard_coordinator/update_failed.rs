use crate::shard_coordinator::ShardCoordinator;
use actor_core::actor::behavior::Behavior;
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::actor_ref::ActorRef;
use actor_core::message::handler::MessageHandler;
use actor_core::Message;

#[derive(Debug, Message, derive_more::Display)]
#[display("UpdateFailed")]
pub(super) struct UpdateFailed;

impl MessageHandler<ShardCoordinator> for UpdateFailed {
    fn handle(
        actor: &mut ShardCoordinator,
        ctx: &mut <ShardCoordinator as Actor>::Context,
        _: Self,
        _: Option<ActorRef>,
        _: &Receive<ShardCoordinator>,
    ) -> anyhow::Result<Behavior<ShardCoordinator>> {
        actor.update(ctx, None);
        Ok(Behavior::same())
    }
}

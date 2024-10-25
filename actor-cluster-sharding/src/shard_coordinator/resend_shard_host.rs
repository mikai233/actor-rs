use crate::shard_coordinator::ShardCoordinator;
use crate::shard_region::ImShardId;
use actor_core::actor::behavior::Behavior;
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::actor_ref::ActorRef;
use actor_core::message::handler::MessageHandler;
use actor_core::Message;

#[derive(Debug, Message, derive_more::Display)]
#[display("ResendShardHost {{ shard: {shard}, region: {region} }}")]
pub(super) struct ResendShardHost {
    pub(super) shard: ImShardId,
    pub(super) region: ActorRef,
}

impl MessageHandler<ShardCoordinator> for ResendShardHost {
    fn handle(
        actor: &mut ShardCoordinator,
        ctx: &mut <ShardCoordinator as Actor>::Context,
        message: Self,
        sender: Option<ActorRef>,
        _: &Receive<ShardCoordinator>,
    ) -> anyhow::Result<Behavior<ShardCoordinator>> {
        if let Some(region) = actor.state.shards.get(&message.shard) {
            if region == &message.region {
                actor.send_host_shard_msg(ctx, message.shard, message.region);
            }
        }
        Ok(Behavior::same())
    }
}

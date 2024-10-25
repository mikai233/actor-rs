use actor_core::message::handler::MessageHandler;

use crate::shard_region::ShardRegion;

#[derive(Debug, Clone, Message, derive_more::Display)]
#[cloneable]
#[display("RegisterRetry")]
pub(super) struct RegisterRetry;

impl MessageHandler<ShardRegion> for RegisterRetry {
    fn handle(
        actor: &mut ShardRegion,
        ctx: &mut <ShardRegion as Actor>::Context,
        message: Self,
        sender: Option<ActorRef>,
        _: &Receive<ShardRegion>,
    ) -> anyhow::Result<Behavior<ShardRegion>> {
        todo!()
    }
}

#[async_trait]
impl Message for RegisterRetry {
    type A = ShardRegion;

    async fn handle(
        self: Box<Self>,
        context: &mut Context,
        actor: &mut Self::A,
    ) -> anyhow::Result<()> {
        if actor.coordinator.is_none() {
            actor.register(context)?;
            actor.scheduler_next_registration(context);
        }
        Ok(())
    }
}

use std::any::type_name;

use anyhow::Context as _;
use async_trait::async_trait;
use tracing::debug;

use actor_core::actor::context::{ActorContext, Context};
use actor_core::EmptyCodec;
use actor_core::ext::option_ext::OptionExt;
use actor_core::Message;

use crate::shard_region::{ImShardId, ShardRegion};
use crate::shard_region::deliver_target::DeliverTarget;

#[derive(Debug, EmptyCodec)]
pub(crate) struct ShardInitialized {
    pub(crate) shard_id: ImShardId,
}

#[async_trait]
impl Message for ShardInitialized {
    type A = ShardRegion;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let shard = context.sender().into_result().context(type_name::<ShardInitialized>())?;
        debug!("{}: Shard was initialized [{}]", actor.type_name, self.shard_id);
        actor.starting_shards.remove(&self.shard_id);
        actor.deliver_buffered_messages(&self.shard_id, DeliverTarget::Shard(shard));
        Ok(())
    }
}
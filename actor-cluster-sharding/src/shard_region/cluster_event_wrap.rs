use async_trait::async_trait;

use actor_cluster::cluster_event::ClusterEvent;
use actor_core::actor::context::ActorContext;
use actor_core::Message;
use actor_derive::EmptyCodec;

use crate::shard_region::ShardRegion;

#[derive(Debug, EmptyCodec)]
pub(super) struct ClusterEventWrap(pub(super) ClusterEvent);

#[async_trait]
impl Message for ClusterEventWrap {
    type A = ShardRegion;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        todo!()
    }
}
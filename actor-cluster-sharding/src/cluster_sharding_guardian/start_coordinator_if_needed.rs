use std::sync::Arc;

use async_trait::async_trait;
use imstr::ImString;

use actor_core::actor::context::ActorContext;
use actor_core::EmptyCodec;
use actor_core::Message;

use crate::cluster_sharding_guardian::ClusterShardingGuardian;
use crate::cluster_sharding_settings::ClusterShardingSettings;
use crate::shard_allocation_strategy::ShardAllocationStrategy;

#[derive(Debug, EmptyCodec)]
pub(crate) struct StartCoordinatorIfNeeded {
    pub(crate) type_name: ImString,
    pub(crate) settings: Arc<ClusterShardingSettings>,
    pub(crate) allocation_strategy: Box<dyn ShardAllocationStrategy>,
}

#[async_trait]
impl Message for StartCoordinatorIfNeeded {
    type A = ClusterShardingGuardian;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> eyre::Result<()> {
        let Self { type_name, settings, allocation_strategy } = *self;
        actor.start_coordinator_if_needed(context, type_name, allocation_strategy, settings)?;
        Ok(())
    }
}
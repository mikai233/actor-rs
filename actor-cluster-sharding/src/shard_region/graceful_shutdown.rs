use std::time::Duration;

use async_trait::async_trait;
use tracing::debug;

use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::coordinated_shutdown::{CoordinatedShutdown, PHASE_CLUSTER_SHARDING_SHUTDOWN_REGION};
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::ext::message_ext::UserMessageExt;
use actor_core::Message;
use actor_derive::EmptyCodec;

use crate::shard_region::graceful_shutdown_timeout::GracefulShutdownTimeout;
use crate::shard_region::ShardRegion;

#[derive(Debug, EmptyCodec)]
pub(super) struct GracefulShutdown;

#[async_trait]
impl Message for GracefulShutdown {
    type A = ShardRegion;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> eyre::Result<()> {
        if actor.preparing_for_shutdown {
            debug!("{}: Skipping graceful shutdown of region and all its shards as cluster is preparing for shutdown", actor.type_name);
            let _ = actor.graceful_shutdown_progress.send(()).await;
            context.stop(context.myself());
        } else {
            debug!("{}: Starting graceful shutdown of region and all its shards", actor.type_name);
            let coord_shutdown = CoordinatedShutdown::get(context.system());
            if coord_shutdown.run_started() {
                let timeout = CoordinatedShutdown::timeout(context.system(), PHASE_CLUSTER_SHARDING_SHUTDOWN_REGION)
                    .expect(&format!("phase {} not found", PHASE_CLUSTER_SHARDING_SHUTDOWN_REGION))
                    .checked_sub(Duration::from_secs(1));
                if let Some(timeout) = timeout {
                    actor.timers.start_single_timer(timeout, GracefulShutdownTimeout.into_dyn(), context.myself().clone());
                }
            }
            drop(coord_shutdown);
            actor.graceful_shutdown_in_progress = true;
            actor.send_graceful_shutdown_to_coordinator_if_in_progress(context)?;
            actor.try_complete_graceful_shutdown_if_in_progress(context);
        }
        Ok(())
    }
}
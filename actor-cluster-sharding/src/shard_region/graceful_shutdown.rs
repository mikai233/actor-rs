use std::time::Duration;

use crate::shard_region::graceful_shutdown_timeout::GracefulShutdownTimeout;
use crate::shard_region::ShardRegion;
use actor_core::actor::behavior::Behavior;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::coordinated_shutdown::{
    CoordinatedShutdown, PHASE_CLUSTER_SHARDING_SHUTDOWN_REGION,
};
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::actor_ref::ActorRef;
use actor_core::message::handler::MessageHandler;
use actor_core::Message;
use tracing::debug;

#[derive(Debug, Message, derive_more::Display)]
#[display("GracefulShutdown")]
pub(super) struct GracefulShutdown;

impl MessageHandler<ShardRegion> for GracefulShutdown {
    fn handle(
        actor: &mut ShardRegion,
        ctx: &mut <ShardRegion as Actor>::Context,
        message: Self,
        sender: Option<ActorRef>,
        _: &Receive<ShardRegion>,
    ) -> anyhow::Result<Behavior<ShardRegion>> {
        if actor.preparing_for_shutdown {
            debug!("{}: Skipping graceful shutdown of region and all its shards as cluster is preparing for shutdown", actor.type_name);
            let _ = actor.graceful_shutdown_progress.send(()).await;
            ctx.stop(ctx.myself());
        } else {
            debug!(
                "{}: Starting graceful shutdown of region and all its shards",
                actor.type_name
            );
            let coord_shutdown = CoordinatedShutdown::get(ctx.system());
            if coord_shutdown.run_started() {
                let timeout = CoordinatedShutdown::timeout(
                    ctx.system(),
                    PHASE_CLUSTER_SHARDING_SHUTDOWN_REGION,
                )
                .expect(&format!(
                    "phase {} not found",
                    PHASE_CLUSTER_SHARDING_SHUTDOWN_REGION
                ))
                .checked_sub(Duration::from_secs(1));
                if let Some(timeout) = timeout {
                    actor.timers.start_single_timer(
                        timeout,
                        GracefulShutdownTimeout,
                        ctx.myself().clone(),
                    );
                }
            }
            drop(coord_shutdown);
            actor.graceful_shutdown_in_progress = true;
            actor.send_graceful_shutdown_to_coordinator_if_in_progress(ctx)?;
            actor.try_complete_graceful_shutdown_if_in_progress(ctx);
        }
        Ok(Behavior::same())
    }
}

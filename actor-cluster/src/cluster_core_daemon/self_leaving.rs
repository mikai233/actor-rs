use std::ops::Not;

use async_trait::async_trait;
use tracing::info;

use actor_core::actor::context::ActorContext;
use actor_core::actor::coordinated_shutdown::{ClusterLeavingReason, CoordinatedShutdown};
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::EmptyCodec;
use actor_core::Message;

use crate::cluster_core_daemon::ClusterCoreDaemon;

#[derive(Debug, EmptyCodec)]
pub(crate) struct SelfLeaving;

#[async_trait]
impl Message for SelfLeaving {
    type A = ClusterCoreDaemon;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> eyre::Result<()> {
        if !actor.exiting_tasks_in_progress {
            actor.exiting_tasks_in_progress = true;
            let coord_shutdown = CoordinatedShutdown::get(context.system());
            if coord_shutdown.run_started().not() {
                info!("Exiting, starting coordinated shutdown");
            }
            let _ = actor.self_exiting.send(()).await;
            tokio::spawn(async move {
                coord_shutdown.run(ClusterLeavingReason).await;
            });
        }
        Ok(())
    }
}
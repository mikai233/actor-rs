use std::ops::Not;

use actor_core::actor::behavior::Behavior;
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::actor_ref::ActorRef;
use actor_core::message::handler::MessageHandler;
use actor_core::Message;
use tracing::info;

use actor_core::actor::coordinated_shutdown::{ClusterLeavingReason, CoordinatedShutdown};
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;

use crate::cluster_core_daemon::ClusterCoreDaemon;

#[derive(Debug, Message, derive_more::Display)]
#[display("SelfLeaving")]
pub(crate) struct SelfLeaving;

impl MessageHandler<ClusterCoreDaemon> for SelfLeaving {
    fn handle(
        actor: &mut ClusterCoreDaemon,
        ctx: &mut <ClusterCoreDaemon as Actor>::Context,
        message: Self,
        sender: Option<ActorRef>,
        _: &Receive<ClusterCoreDaemon>,
    ) -> anyhow::Result<Behavior<ClusterCoreDaemon>> {
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
        Ok(Behavior::same())
    }
}

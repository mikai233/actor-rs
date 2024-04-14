use std::any::type_name;

use async_trait::async_trait;
use tracing::debug;

use actor_core::{CodecMessage, DynMessage, Message};
use actor_core::actor::context::ActorContext;
use actor_core::EmptyCodec;
use actor_core::message::terminated::Terminated;

use crate::cluster_core_daemon::ClusterCoreDaemon;
use crate::cluster_core_supervisor::ClusterCoreSupervisor;

#[derive(Debug, EmptyCodec)]
pub(super) struct CoreDaemonTerminated(pub(super) Terminated);

impl CoreDaemonTerminated {
    pub(super) fn new(terminated: Terminated) -> DynMessage {
        Self(terminated).into_dyn()
    }
}

#[async_trait]
impl Message for CoreDaemonTerminated {
    type A = ClusterCoreSupervisor;

    async fn handle(self: Box<Self>, _context: &mut ActorContext, _actor: &mut Self::A) -> eyre::Result<()> {
        debug!("{} {} terminated", type_name::<ClusterCoreDaemon>(), self.0.actor);
        Ok(())
    }
}
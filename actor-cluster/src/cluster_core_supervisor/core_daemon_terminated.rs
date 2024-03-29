use async_trait::async_trait;
use tracing::debug;

use actor_core::{DynMessage, Message};
use actor_core::actor::context::{ActorContext, Context};
use actor_core::ext::message_ext::UserMessageExt;
use actor_core::ext::type_name_of;
use actor_core::message::terminated::Terminated;
use actor_derive::EmptyCodec;

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

    async fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
        debug!("{} {} terminated", type_name_of::<ClusterCoreDaemon>(), context.myself());
        Ok(())
    }
}
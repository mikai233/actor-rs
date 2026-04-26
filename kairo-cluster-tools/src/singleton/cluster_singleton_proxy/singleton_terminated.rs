use async_trait::async_trait;
use tracing::debug;

use kairo_core::EmptyCodec;
use kairo_core::actor::context::ActorContext;
use kairo_core::message::terminated::Terminated;
use kairo_core::{CodecMessage, DynMessage, Message};

use crate::singleton::cluster_singleton_proxy::ClusterSingletonProxy;

#[derive(Debug, EmptyCodec)]
pub(super) struct SingletonTerminated(pub(super) Terminated);

impl SingletonTerminated {
    pub(super) fn new(terminated: Terminated) -> DynMessage {
        Self(terminated).into_dyn()
    }
}

#[async_trait]
impl Message for SingletonTerminated {
    type A = ClusterSingletonProxy;

    async fn handle(
        self: Box<Self>,
        _context: &mut ActorContext,
        actor: &mut Self::A,
    ) -> anyhow::Result<()> {
        if let Some(singleton) = &actor.singleton
            && *singleton == *self.0
        {
            debug!("singleton {} terminated", singleton);
            actor.singleton = None;
        }
        Ok(())
    }
}

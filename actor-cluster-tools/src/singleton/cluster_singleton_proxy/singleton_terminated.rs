use async_trait::async_trait;
use tracing::debug;

use actor_core::{DynMessage, Message};
use actor_core::actor::context::ActorContext;
use actor_core::ext::message_ext::UserMessageExt;
use actor_core::message::terminated::Terminated;
use actor_derive::EmptyCodec;

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

    async fn handle(self: Box<Self>, _context: &mut ActorContext, actor: &mut Self::A) -> eyre::Result<()> {
        if let Some(singleton) = &actor.singleton {
            if *singleton == *self.0 {
                debug!("singleton {} terminated", singleton);
                actor.singleton = None;
            }
        }
        Ok(())
    }
}
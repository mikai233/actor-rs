use async_trait::async_trait;
use tracing::debug;

use actor_core::actor::context::ActorContext;
use actor_core::actor_ref::ActorRef;
use actor_core::Message;
use actor_core::message::terminated::Terminated;
use actor_derive::EmptyCodec;

use crate::singleton::cluster_singleton_proxy::ClusterSingletonProxy;

#[derive(Debug, EmptyCodec)]
pub(super) struct SingletonTerminated(pub(super) ActorRef);

#[async_trait]
impl Message for SingletonTerminated {
    type A = ClusterSingletonProxy;

    async fn handle(self: Box<Self>, _context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        if let Some(singleton) = &actor.singleton {
            if *singleton == self.0 {
                debug!("singleton {} terminated", singleton);
                actor.singleton = None;
            }
        }
        Ok(())
    }
}

impl Terminated for SingletonTerminated {
    fn actor(&self) -> &ActorRef {
        &self.0
    }
}
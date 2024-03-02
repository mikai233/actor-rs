use async_trait::async_trait;
use tracing::debug;

use actor_core::actor::actor_ref::ActorRef;
use actor_core::actor::context::ActorContext;
use actor_core::Message;
use actor_core::message::terminated::Terminated;
use actor_derive::EmptyCodec;

use crate::singleton::cluster_singleton_manager::ClusterSingletonManager;

#[derive(Debug, EmptyCodec)]
pub(super) struct SingletonTerminated(pub(super) ActorRef);

#[async_trait]
impl Message for SingletonTerminated {
    type A = ClusterSingletonManager;

    async fn handle(self: Box<Self>, _context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        debug!("singleton manager watch singleton actor {} terminated", self.0);
        actor.singleton = None;
        if let Some(notifier) = actor.singleton_shutdown_notifier.take() {
            let _ = notifier.send(());
        }
        actor.unlock().await?;
        Ok(())
    }
}

impl Terminated for SingletonTerminated {
    fn actor(&self) -> &ActorRef {
        &self.0
    }
}
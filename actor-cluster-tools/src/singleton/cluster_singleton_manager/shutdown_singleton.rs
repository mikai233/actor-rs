use async_trait::async_trait;
use tokio::sync::oneshot::Sender;
use tracing::debug;

use actor_core::actor::context::Context;
use actor_core::actor_ref::ActorRef;
use actor_core::EmptyCodec;
use actor_core::Message;

use crate::singleton::cluster_singleton_manager::ClusterSingletonManager;

#[derive(Debug, EmptyCodec)]
pub(super) struct ShutdownSingleton(pub(super) Sender<()>);

#[async_trait]
impl Message for ShutdownSingleton {
    type A = ClusterSingletonManager;

    async fn handle(self: Box<Self>, _context: &mut Context, actor: &mut Self::A) -> anyhow::Result<()> {
        if let Some(singleton) = &actor.singleton {
            actor.singleton_shutdown_notifier = Some(self.0);
            let termination = actor.termination_message.dyn_clone()?;
            debug!("send termination message {} to singleton {}", termination.name(), singleton);
            singleton.tell(termination, ActorRef::no_sender());
        } else {
            let _ = self.0.send(());
        }
        Ok(())
    }
}
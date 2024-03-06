use async_trait::async_trait;
use tokio::sync::oneshot::Sender;

use actor_core::actor::context::ActorContext;
use actor_core::actor_ref::ActorRef;
use actor_core::Message;
use actor_derive::EmptyCodec;

use crate::singleton::cluster_singleton_manager::ClusterSingletonManager;

#[derive(Debug, EmptyCodec)]
pub(super) struct ShutdownSingleton(pub(super) Sender<()>);

#[async_trait]
impl Message for ShutdownSingleton {
    type A = ClusterSingletonManager;

    async fn handle(self: Box<Self>, _context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        if let Some(singleton) = &actor.singleton {
            actor.singleton_shutdown_notifier = Some(self.0);
            singleton.tell(actor.termination_message.dyn_clone().unwrap(), ActorRef::no_sender());
        } else {
            let _ = self.0.send(());
        }
        Ok(())
    }
}
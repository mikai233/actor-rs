use async_trait::async_trait;

use actor_core::actor::context::{ActorContext, Context};
use actor_core::EmptyCodec;
use actor_core::Message;
use actor_core::message::identify::ActorIdentity;

use crate::singleton::cluster_singleton_proxy::ClusterSingletonProxy;
use crate::singleton::cluster_singleton_proxy::singleton_terminated::SingletonTerminated;

#[derive(Debug, EmptyCodec)]
pub(super) struct ActorIdentityWrap(pub(super) ActorIdentity);

#[async_trait]
impl Message for ActorIdentityWrap {
    type A = ClusterSingletonProxy;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> eyre::Result<()> {
        if let Some(singleton) = self.0.actor_ref {
            if !context.is_watching(&singleton) {
                context.watch(singleton.clone(), SingletonTerminated::new)?;
            }
            actor.singleton = Some(singleton);
            actor.cancel_timer();
            actor.send_buffered();
        }
        Ok(())
    }
}
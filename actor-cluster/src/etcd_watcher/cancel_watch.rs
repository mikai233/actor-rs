use async_trait::async_trait;

use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::Message;
use actor_derive::EmptyCodec;

use crate::etcd_watcher::EtcdWatcher;

/// cancel key watcher and stop watch actor
#[derive(Debug, EmptyCodec)]
pub struct CancelWatch;

#[async_trait]
impl Message for CancelWatch {
    type A = EtcdWatcher;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        if let Some(watcher) = &mut actor.watcher {
            watcher.cancel().await?;
            context.stop(context.myself());
        }
        Ok(())
    }
}
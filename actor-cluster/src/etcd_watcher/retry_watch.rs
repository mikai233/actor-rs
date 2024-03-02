use async_trait::async_trait;

use actor_core::actor::context::ActorContext;
use actor_core::Message;
use actor_derive::EmptyCodec;

use crate::etcd_watcher::EtcdWatcher;

#[derive(Debug, EmptyCodec)]
pub(super) struct RetryWatch;

#[async_trait]
impl Message for RetryWatch {
    type A = EtcdWatcher;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        actor.watch(context).await;
        Ok(())
    }
}
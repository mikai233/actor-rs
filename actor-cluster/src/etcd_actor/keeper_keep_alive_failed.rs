use async_trait::async_trait;

use actor_core::{DynMessage, Message};
use actor_core::actor::context::{ActorContext, Context};
use actor_derive::EmptyCodec;

use crate::etcd_actor::EtcdActor;
use crate::etcd_actor::keep_alive_failed::KeepAliveFailed;

#[derive(Debug, EmptyCodec)]
pub(super) struct KeeperKeepAliveFailed {
    pub id: i64,
    pub error: etcd_client::Error,
}

#[async_trait]
impl Message for KeeperKeepAliveFailed {
    type A = EtcdActor;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        if let Some(lease) = actor.lease.remove(&self.id) {
            let keep_alive_failed = KeepAliveFailed {
                id: self.id,
                error: Some(self.error),
            };
            lease.watcher.tell(DynMessage::orphan(keep_alive_failed), Some(context.myself().clone()));
        }
        Ok(())
    }
}
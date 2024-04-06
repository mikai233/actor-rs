use async_trait::async_trait;

use actor_core::actor::context::ActorContext;
use actor_core::Message;
use actor_derive::EmptyCodec;

use crate::etcd_actor::EtcdActor;

#[derive(Debug, EmptyCodec)]
pub(super) struct KeeperKeepAliveFailed {
    pub id: i64,
    pub error: etcd_client::Error,
}

#[async_trait]
impl Message for KeeperKeepAliveFailed {
    type A = EtcdActor;

    async fn handle(self: Box<Self>, _context: &mut ActorContext, actor: &mut Self::A) -> eyre::Result<()> {
        if let Some(lease) = actor.lease.remove(&self.id) {
            EtcdActor::keep_alive_failed(self.id, &lease.applicant, Some(self.error));
        }
        Ok(())
    }
}
use async_trait::async_trait;

use kairo_core::EmptyCodec;
use kairo_core::Message;
use kairo_core::actor::context::ActorContext;

use crate::etcd_actor::EtcdActor;

#[derive(Debug, EmptyCodec)]
pub(super) struct KeeperKeepAliveFailed {
    pub id: i64,
    pub error: etcd_client::Error,
}

#[async_trait]
impl Message for KeeperKeepAliveFailed {
    type A = EtcdActor;

    async fn handle(
        self: Box<Self>,
        _context: &mut ActorContext,
        actor: &mut Self::A,
    ) -> anyhow::Result<()> {
        if let Some(lease) = actor.lease.remove(&self.id) {
            EtcdActor::keep_alive_failed(self.id, &lease.applicant, Some(self.error));
        }
        Ok(())
    }
}

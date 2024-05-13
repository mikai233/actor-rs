use async_trait::async_trait;
use tracing::debug;

use actor_core::actor::context::ActorContext;
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::EmptyCodec;
use actor_core::Message;

use crate::etcd_actor::EtcdActor;

#[derive(Debug, EmptyCodec)]
pub struct CancelKeepAlive(pub i64);

#[async_trait]
impl Message for CancelKeepAlive {
    type A = EtcdActor;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        if let Some(lease) = actor.lease.remove(&self.0) {
            context.stop(&lease.keeper);
            debug!("cancel keep alive with lease {}", self.0);
        }
        Ok(())
    }
}
use async_trait::async_trait;
use tracing::error;

use kairo_cluster::etcd_client;
use kairo_core::EmptyCodec;
use kairo_core::Message;
use kairo_core::actor::context::ActorContext;

use crate::singleton::cluster_singleton_manager::ClusterSingletonManager;

#[derive(Debug, EmptyCodec)]
pub(super) struct LockFailed {
    pub(super) path: String,
    pub(super) error: etcd_client::Error,
}

#[async_trait]
impl Message for LockFailed {
    type A = ClusterSingletonManager;

    async fn handle(
        self: Box<Self>,
        context: &mut ActorContext,
        actor: &mut Self::A,
    ) -> anyhow::Result<()> {
        let Self { path, error } = *self;
        error!("lock singleton {} failed {:?}, retry it", path, error);
        actor.lock(context)?;
        Ok(())
    }
}

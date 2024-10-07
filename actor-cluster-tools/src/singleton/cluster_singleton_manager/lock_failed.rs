use async_trait::async_trait;
use tracing::error;

use actor_cluster::etcd_client;
use actor_core::actor::context::ActorContext1;
use actor_core::EmptyCodec;
use actor_core::Message;

use crate::singleton::cluster_singleton_manager::ClusterSingletonManager;

#[derive(Debug, EmptyCodec)]
pub(super) struct LockFailed {
    pub(super) path: String,
    pub(super) error: etcd_client::Error,
}

#[async_trait]
impl Message for LockFailed {
    type A = ClusterSingletonManager;

    async fn handle(self: Box<Self>, context: &mut ActorContext1, actor: &mut Self::A) -> anyhow::Result<()> {
        let Self { path, error } = *self;
        error!("lock singleton {} failed {:?}, retry it", path, error);
        actor.lock(context)?;
        Ok(())
    }
}
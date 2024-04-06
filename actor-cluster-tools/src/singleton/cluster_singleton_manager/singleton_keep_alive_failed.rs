use std::time::Duration;

use async_trait::async_trait;
use tracing::warn;

use actor_cluster::etcd_actor::keep_alive::KeepAliveFailed;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor_path::TActorPath;
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::actor_ref::ActorRefExt;
use actor_core::Message;
use actor_derive::EmptyCodec;

use crate::singleton::cluster_singleton_manager::ClusterSingletonManager;

#[derive(Debug, EmptyCodec)]
pub(super) struct SingletonKeepAliveFailed(pub(super) Option<KeepAliveFailed>);

#[async_trait]
impl Message for SingletonKeepAliveFailed {
    type A = ClusterSingletonManager;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> eyre::Result<()> {
        match actor.keep_alive().await {
            Ok(lease_id) => {
                actor.lease_id = lease_id;
                if let Some(handle) = actor.lock_handle.take() {
                    handle.abort();
                }
                //TODO stop singleton
                actor.lock(context);
            }
            Err(error) => {
                let myself = context.myself().clone();
                let name = myself.path().name();
                let retry = Duration::from_secs(1);
                warn!("{} keep alive failed {:?}, retry after {:?}", name, error, retry);
                context.system().scheduler.schedule_once(retry, move || {
                    myself.cast_ns(SingletonKeepAliveFailed(None));
                });
            }
        }
        Ok(())
    }
}
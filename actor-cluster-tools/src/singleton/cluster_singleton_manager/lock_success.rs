use async_trait::async_trait;
use tracing::{error, info};

use actor_core::actor::context::{ActorContext1, ActorContext};
use actor_core::actor_path::TActorPath;
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::EmptyCodec;
use actor_core::Message;

use crate::singleton::cluster_singleton_manager::ClusterSingletonManager;
use crate::singleton::cluster_singleton_manager::singleton_terminated::SingletonTerminated;

#[derive(Debug, EmptyCodec)]
pub(super) struct LockSuccess(pub(super) Vec<u8>);

#[async_trait]
impl Message for LockSuccess {
    type A = ClusterSingletonManager;

    async fn handle(self: Box<Self>, context: &mut ActorContext1, actor: &mut Self::A) -> anyhow::Result<()> {
        actor.lock_key = Some(self.0);
        match context.spawn(actor.singleton_props.props(()), actor.singleton_name()) {
            Ok(singleton) => {
                context.watch_with(singleton.clone(), SingletonTerminated::new)?;
                info!("singleton manager start singleton actor {}", singleton);
                actor.singleton = Some(singleton);
            }
            Err(error) => {
                let name = context.myself().path().name();
                error!("spawn singleton {} error {:?}", name, error);
                actor.unlock().await?;
            }
        };
        Ok(())
    }
}
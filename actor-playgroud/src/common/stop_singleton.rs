use async_trait::async_trait;
use tracing::info;

use actor_core::actor::context::{ActorContext1, ActorContext};
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::CEmptyCodec;
use actor_core::Message;

use crate::common::singleton_actor::SingletonActor;

#[derive(Debug, Clone, CEmptyCodec)]
pub struct StopSingleton;

#[async_trait]
impl Message for StopSingleton {
    type A = SingletonActor;

    async fn handle(self: Box<Self>, context: &mut ActorContext1, _actor: &mut Self::A) -> anyhow::Result<()> {
        info!("stop singleton {}", context.myself());
        context.stop(context.myself());
        Ok(())
    }
}
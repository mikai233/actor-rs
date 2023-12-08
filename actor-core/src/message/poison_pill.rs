use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::debug;

use actor_derive::SystemMessageCodec;
use crate::actor::actor_ref_factory::ActorRefFactory;

use crate::actor::context::{ActorContext, Context};
use crate::SystemMessage;

#[derive(Debug, Serialize, Deserialize, SystemMessageCodec)]
pub struct PoisonPill;

#[async_trait]
impl SystemMessage for PoisonPill {
    async fn handle(self: Box<Self>, context: &mut ActorContext) -> anyhow::Result<()> {
        debug!("{} receive PoisonPill", context.myself());
        context.stop(context.myself());
        Ok(())
    }
}
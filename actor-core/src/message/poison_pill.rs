use async_trait::async_trait;
use bincode::{Decode, Encode};
use tracing::debug;

use actor_derive::SystemCodec;

use crate::{Actor, SystemMessage};
use crate::actor::actor_ref_factory::ActorRefFactory;
use crate::actor::context::{ActorContext, Context};

#[derive(Debug, Encode, Decode, SystemCodec)]
pub struct PoisonPill;

#[async_trait]
impl SystemMessage for PoisonPill {
    async fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut dyn Actor) -> anyhow::Result<()> {
        debug!("{} receive PoisonPill", context.myself());
        context.stop(context.myself());
        Ok(())
    }
}
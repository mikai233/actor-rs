use async_trait::async_trait;
use bincode::{Decode, Encode};

use actor_derive::{CSystemCodec, OrphanCodec};

use crate::{Actor, SystemMessage};
use crate::actor::context::{ActorContext, Context};
use crate::actor_ref::{ActorRef, ActorRefExt};
use crate::ext::option_ext::OptionExt;

#[derive(Debug, Clone, Encode, Decode, CSystemCodec)]
pub struct Identify;

#[async_trait]
impl SystemMessage for Identify {
    async fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut dyn Actor) -> anyhow::Result<()> {
        let myself = context.myself().clone();
        let actor_identify = ActorIdentity {
            actor_ref: Some(myself),
        };
        context.sender().foreach(|sender| {
            sender.cast_orphan_ns(actor_identify);
        });
        Ok(())
    }
}

#[derive(Debug, Encode, Decode, OrphanCodec)]
pub struct ActorIdentity {
    pub actor_ref: Option<ActorRef>,
}
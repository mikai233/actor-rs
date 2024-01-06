use async_trait::async_trait;
use bincode::{Decode, Encode};

use actor_derive::{CSystemCodec, OrphanCodec};

use crate::{Actor, SystemMessage};
use crate::actor::actor_ref::{ActorRef, ActorRefExt};
use crate::actor::context::{ActorContext, Context};
use crate::ext::option_ext::OptionExt;

#[derive(Debug, Clone, Encode, Decode, CSystemCodec)]
pub(crate) struct Identify;

#[async_trait]
impl SystemMessage for Identify {
    async fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut dyn Actor) -> anyhow::Result<()> {
        let myself = context.myself().clone();
        let actor_identify = ActorIdentity {
            actor_ref: Some(myself),
        };
        context.sender().foreach(|sender| {
            sender.resp(actor_identify);
        });
        Ok(())
    }
}

#[derive(Debug, Encode, Decode, OrphanCodec)]
pub(crate) struct ActorIdentity {
    pub(crate) actor_ref: Option<ActorRef>,
}
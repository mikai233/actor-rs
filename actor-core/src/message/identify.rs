use async_trait::async_trait;
use bincode::{Decode, Encode};

use actor_derive::SystemCodec;

use crate::{Actor, SystemMessage};
use crate::actor::actor_ref::{ActorRef, ActorRefSystemExt};
use crate::actor::context::{ActorContext, Context};
use crate::ext::option_ext::OptionExt;

#[derive(Debug, Encode, Decode, SystemCodec)]
pub(crate) struct Identify;

#[async_trait]
impl SystemMessage for Identify {
    async fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut dyn Actor) -> anyhow::Result<()> {
        let myself = context.myself().clone();
        let actor_identify = ActorIdentity {
            actor_ref: Some(myself),
        };
        context.sender().foreach(|sender| {
            sender.cast_system(actor_identify, None);
        });
        Ok(())
    }
}

#[derive(Debug, Encode, Decode, SystemCodec)]
pub(crate) struct ActorIdentity {
    pub(crate) actor_ref: Option<ActorRef>,
}

#[async_trait]
impl SystemMessage for ActorIdentity {
    async fn handle(self: Box<Self>, _context: &mut ActorContext, _actor: &mut dyn Actor) -> anyhow::Result<()> {
        todo!()
    }
}
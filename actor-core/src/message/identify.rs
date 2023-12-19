use async_trait::async_trait;
use bincode::{Decode, Encode};

use actor_derive::SystemMessageCodec;

use crate::{Actor, CodecMessage, SystemMessage};
use crate::actor::actor_ref::{ActorRef, ActorRefSystemExt};
use crate::actor::context::{ActorContext, Context};
use crate::ext::option_ext::OptionExt;

#[derive(Debug, Encode, Decode, SystemMessageCodec)]
pub(crate) struct Identify;

#[async_trait]
impl SystemMessage for Identify {
    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut dyn Actor) -> anyhow::Result<()> {
        let myself = context.myself().clone();
        let actor_identify = ActorIdentify {
            actor_ref: myself,
        };
        context.sender().foreach(|sender| {
            sender.cast_system(actor_identify, None);
        });
        Ok(())
    }
}

#[derive(Debug, Encode, Decode, SystemMessageCodec)]
pub(crate) struct ActorIdentify {
    pub(crate) actor_ref: ActorRef,
}

#[async_trait]
impl SystemMessage for ActorIdentify {
    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut dyn Actor) -> anyhow::Result<()> {
        todo!()
    }
}
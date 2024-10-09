use actor_derive::Message;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::actor::context::{Context, ActorContext};
use crate::actor_ref::ActorRef;
use crate::ext::option_ext::OptionExt;

#[derive(Debug, Copy, Clone, Serialize, Deserialize, Message, derive_more::Display)]
#[cloneable]
#[display("Identify")]
pub struct Identify;

#[async_trait]
impl SystemMessage for Identify {
    async fn handle(self: Box<Self>, context: &mut Context, _actor: &mut dyn Actor) -> anyhow::Result<()> {
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

#[derive(Debug, Clone, Serialize, Deserialize, Message, derive_more::Display)]
#[cloneable]
#[display("ActorIdentity {{ actor_ref: {actor_ref} }}")]
pub struct ActorIdentity {
    pub actor_ref: Option<ActorRef>,
}
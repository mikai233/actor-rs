use std::borrow::Cow;
use std::fmt::Debug;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::system_message_decoder;
use crate::actor::{MessageDecoder, SystemMessage};
use crate::actor::context::ActorContext;
use crate::actor_ref::SerializedActorRef;
use crate::ext::encode_bytes;

#[derive(Debug, Serialize, Deserialize)]
pub enum ActorRemoteMessage {
    User {
        name: Cow<'static, str>,
        message: Vec<u8>,
    },
    System {
        name: Cow<'static, str>,
        message: Vec<u8>,
    },
}

impl ActorRemoteMessage {
    pub fn name(&self) -> &str {
        match self {
            ActorRemoteMessage::User { name, .. } => name,
            ActorRemoteMessage::System { name, .. } => name,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) struct Terminate;

#[async_trait(? Send)]
impl SystemMessage for Terminate {
    fn decoder() -> Box<dyn MessageDecoder> where Self: Sized {
        system_message_decoder!(Terminate)
    }

    fn encode(&self) -> anyhow::Result<Vec<u8>> {
        encode_bytes(self)
    }

    async fn handle(self: Box<Self>, context: &mut ActorContext) -> anyhow::Result<()> {
        context.handle_terminate();
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) struct Terminated(SerializedActorRef);

#[async_trait(? Send)]
impl SystemMessage for Terminated {
    fn decoder() -> Box<dyn MessageDecoder> where Self: Sized {
        system_message_decoder!(Terminated)
    }

    fn encode(&self) -> anyhow::Result<Vec<u8>> {
        encode_bytes(self)
    }

    async fn handle(self: Box<Self>, context: &mut ActorContext) -> anyhow::Result<()> {
        todo!()
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) struct DeathWatchNotification(pub(crate) SerializedActorRef);

#[async_trait(? Send)]
impl SystemMessage for DeathWatchNotification {
    fn decoder() -> Box<dyn MessageDecoder> where Self: Sized {
        system_message_decoder!(DeathWatchNotification)
    }

    fn encode(&self) -> anyhow::Result<Vec<u8>> {
        encode_bytes(self)
    }

    async fn handle(self: Box<Self>, context: &mut ActorContext) -> anyhow::Result<()> {
        todo!()
    }
}

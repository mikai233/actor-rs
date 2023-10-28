use std::any::Any;
use std::fmt::{Debug, Formatter};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use crate::actor::Message;

use crate::actor_ref::{ActorRef, SerializedActorRef};
use crate::ext::encode_bytes;

pub trait MessageID: Sized {
    fn id() -> u32;
}

#[derive(Debug)]
pub enum ActorMessage {
    Local(ActorLocalMessage),
    Remote(ActorRemoteMessage),
}

impl ActorMessage {
    pub fn local<M>(message: M) -> ActorMessage where M: Message {
        let name = std::any::type_name::<M>();
        let boxed = Box::new(message);
        let local = ActorLocalMessage {
            name,
            inner: boxed,
        };
        ActorMessage::Local(local)
    }

    pub fn remote<M>(message: &M) -> anyhow::Result<ActorMessage> where M: Message + MessageID + Serialize + DeserializeOwned {
        let name = std::any::type_name::<M>();
        let id = M::id();
        let body = encode_bytes(message)?;
        let remote = ActorRemoteMessage {
            name,
            id,
            body,
        };
        let remote = ActorMessage::Remote(remote);
        Ok(remote)
    }

    pub fn name(&self) -> &'static str {
        match self {
            ActorMessage::Local(l) => {
                l.name
            }
            ActorMessage::Remote(r) => {
                r.name
            }
        }
    }
}

pub struct ActorLocalMessage {
    pub name: &'static str,
    pub inner: Box<dyn Any + Send + 'static>,
}

impl Debug for ActorLocalMessage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(&"ActorLocalMessage")
            .field("name", &self.name)
            .field(&"inner", &"..")
            .finish()
    }
}

#[derive(Debug)]
pub struct ActorRemoteMessage {
    pub name: &'static str,
    pub id: u32,
    pub body: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) enum Signal {
    Stop,
    Terminated(SerializedActorRef),
}
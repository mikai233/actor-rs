use std::any::Any;
use std::fmt::{Debug, Formatter};

use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;

use crate::actor::Message;
use crate::actor_ref::{ActorRef, SerializedActorRef};
use crate::ext::encode_bytes;

#[derive(Debug)]
pub enum ActorMessage {
    Local(ActorLocalMessage),
    Remote(ActorRemoteMessage),
}

impl ActorMessage {
    pub fn local<M>(message: M) -> ActorMessage
        where
            M: Message,
    {
        let name = std::any::type_name::<M>();
        let boxed = Box::new(message);
        let local = ActorLocalMessage::User { name, inner: boxed };
        ActorMessage::Local(local)
    }

    pub(crate) fn local_system(message: ActorSystemMessage) -> ActorMessage {
        let local = ActorLocalMessage::System { message };
        ActorMessage::Local(local)
    }

    pub fn remote<M>(message: &M) -> anyhow::Result<ActorMessage>
        where
            M: Message + Serialize + DeserializeOwned,
    {
        let name = std::any::type_name::<M>();
        let body = encode_bytes(message)?;
        let remote = ActorRemoteMessage::User {
            name,
            message: body,
        };
        let remote = ActorMessage::Remote(remote);
        Ok(remote)
    }

    pub(crate) fn remote_system(message: ActorRemoteSystemMessage) -> ActorMessage {
        let remote = ActorRemoteMessage::System { message };
        ActorMessage::Remote(remote)
    }

    pub fn name(&self) -> &str {
        match self {
            ActorMessage::Local(l) => l.name(),
            ActorMessage::Remote(r) => r.name(),
        }
    }
}

pub enum ActorLocalMessage {
    User {
        name: &'static str,
        inner: Box<dyn Any + Send + 'static>,
    },
    System {
        message: ActorSystemMessage,
    },
}

impl ActorLocalMessage {
    pub fn name(&self) -> &str {
        match self {
            ActorLocalMessage::User { name, .. } => name,
            ActorLocalMessage::System { message } => message.name(),
        }
    }
}

impl Debug for ActorLocalMessage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ActorLocalMessage::User { name, .. } => f
                .debug_struct(&"ActorLocalMessage::User")
                .field("name", name)
                .field("inner", &"..")
                .finish(),
            ActorLocalMessage::System { message } => f
                .debug_struct(&"ActorLocalMessage::System")
                .field("message", message)
                .finish(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ActorRemoteMessage {
    User {
        name: &'static str,
        message: Vec<u8>,
    },
    System {
        message: ActorRemoteSystemMessage,
    },
}

impl ActorRemoteMessage {
    pub fn name(&self) -> &str {
        match self {
            ActorRemoteMessage::User { name, .. } => name,
            ActorRemoteMessage::System { message } => message.name(),
        }
    }
}

#[derive(Debug)]
pub enum ActorSystemMessage {
    DeathWatchNotification {
        actor: ActorRef,
    }
}

impl ActorSystemMessage {
    pub(crate) fn name(&self) -> &str {
        match self { ActorSystemMessage::DeathWatchNotification { .. } => { "DeathWatchNotification" } }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ActorRemoteSystemMessage {
    Terminate,
    Terminated(SerializedActorRef),
    Watch {
        watchee: SerializedActorRef,
        watcher: SerializedActorRef,
    },
    UnWatch {
        watchee: SerializedActorRef,
    },
}

impl ActorRemoteSystemMessage {
    pub(crate) fn name(&self) -> &str {
        match self {
            ActorRemoteSystemMessage::Terminate => "Terminate",
            ActorRemoteSystemMessage::Terminated(_) => "Terminated",
            ActorRemoteSystemMessage::Watch { watchee, watcher } => todo!(),
            ActorRemoteSystemMessage::UnWatch { watchee } => todo!(),
        }
    }
}

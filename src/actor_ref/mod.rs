use std::fmt::{Debug, Display, Formatter};

use enum_dispatch::enum_dispatch;
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;

use crate::actor::Message;
use crate::actor_path::ActorPath;
use crate::actor_ref::dead_letter_ref::DeadLetterActorRef;
use crate::actor_ref::local_ref::LocalActorRef;
use crate::actor_ref::remote_ref::RemoteActorRef;
use crate::address::Address;
use crate::message::{ActorMessage, MessageID};
use crate::system::ActorSystem;

pub mod local_ref;
pub mod remote_ref;
pub mod dead_letter_ref;

#[enum_dispatch]
#[derive(Debug, Clone)]
pub enum ActorRef {
    LocalActorRef,
    RemoteActorRef,
    DeadLetterActorRef,
}

impl ActorRef {
    fn no_sender() -> Option<ActorRef> {
        None
    }
}

#[enum_dispatch(ActorRef)]
pub trait TActorRef: Debug + Send + Sync + 'static {
    fn path(&self) -> &ActorPath;
    fn tell(&self, message: ActorMessage, sender: Option<ActorRef>);
}

impl Display for ActorRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let path = self.path();
        let uid = path.uid();
        write!(f, "Actor[{}#{}]", path, uid)
    }
}

impl<T: ?Sized> ActorRefExt for T where T: TActorRef {}

pub trait ActorRefExt: TActorRef {
    fn tell_local<M>(&self, message: M, sender: Option<ActorRef>) where M: Message {
        let local = ActorMessage::local(message);
        self.tell(local, sender);
    }
    fn tell_remote<M>(&self, message: &M, sender: Option<ActorRef>) -> anyhow::Result<()> where M: Message + MessageID + Serialize + DeserializeOwned {
        let remote = ActorMessage::remote(message)?;
        self.tell(remote, sender);
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedActorRef {
    pub address: Address,
    pub path: Vec<String>,
    pub uid: i32,
}

impl Display for SerializedActorRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let path = self.path.join("/");
        write!(f, "SerializedActorRef[{}{}#{}]", self.address, path, self.uid)
    }
}
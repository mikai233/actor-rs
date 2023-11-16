use std::collections::BTreeMap;
use std::fmt::{Debug, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::sync::RwLock;

use enum_dispatch::enum_dispatch;
use serde::{Deserialize, Serialize};

use crate::actor::{AsyncMessage, DeferredMessage, DynamicMessage, Message, SystemMessage};
use crate::actor_path::ActorPath;
use crate::actor_path::TActorPath;
use crate::actor_ref::dead_letter_ref::DeadLetterActorRef;
use crate::actor_ref::deferred_ref::DeferredActorRef;
use crate::actor_ref::local_ref::LocalActorRef;
use crate::actor_ref::remote_ref::RemoteActorRef;
use crate::actor_ref::virtual_path_container::VirtualPathContainer;
use crate::cell::ActorCell;
use crate::system::ActorSystem;

pub mod dead_letter_ref;
pub mod local_ref;
pub mod remote_ref;
pub(crate) mod virtual_path_container;
pub mod deferred_ref;

#[enum_dispatch]
#[derive(Clone)]
pub enum ActorRef {
    LocalActorRef,
    RemoteActorRef,
    DeadLetterActorRef,
    VirtualPathContainer,
    DeferredActorRef,
}

impl Debug for ActorRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ActorRef::LocalActorRef(l) => f
                .debug_struct("ActorRef::LocalActorRef")
                .field("path", l.path())
                .finish_non_exhaustive(),
            ActorRef::RemoteActorRef(r) => f
                .debug_struct("ActorRef::RemoteActorRef")
                .field("path", r.path())
                .finish_non_exhaustive(),
            ActorRef::DeadLetterActorRef(d) => f
                .debug_struct("ActorRef::DeadLetterActorRef")
                .field("path", d.path())
                .finish_non_exhaustive(),
            ActorRef::VirtualPathContainer(v) => f
                .debug_struct("ActorRef::VirtualPathContainer")
                .field("path", v.path())
                .finish_non_exhaustive(),
            ActorRef::DeferredActorRef(d) => f.
                debug_struct("ActorRef::DeferredActorRef")
                .field("path", d.path())
                .finish_non_exhaustive(),
        }
    }
}

impl ActorRef {
    fn no_sender() -> Option<ActorRef> {
        None
    }

    pub(crate) fn local_or_panic(&self) -> &LocalActorRef {
        match self {
            ActorRef::LocalActorRef(l) => l,
            _ => panic!("expect LocalActorRef"),
        }
    }
}

#[enum_dispatch(ActorRef)]
pub trait TActorRef: Debug + Send + 'static {
    fn system(&self) -> ActorSystem;
    fn path(&self) -> &ActorPath;
    fn tell(&self, message: DynamicMessage, sender: Option<ActorRef>);
    fn stop(&self);
    fn parent(&self) -> Option<&ActorRef>;
    fn get_child<I>(&self, names: I) -> Option<ActorRef>
        where
            I: IntoIterator<Item=String>;
}

impl Display for ActorRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let path = self.path();
        let uid = path.uid();
        if uid == ActorPath::undefined_uid() {
            write!(f, "Actor[{}]", path)
        } else {
            write!(f, "Actor[{}#{}]", path, uid)
        }
    }
}

impl PartialEq for ActorRef {
    fn eq(&self, other: &Self) -> bool {
        self.path() == other.path()
    }
}

impl Eq for ActorRef {}

impl Hash for ActorRef {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.path().hash(state)
    }
}

impl<T: ?Sized> ActorRefExt for T where T: TActorRef {}

pub trait ActorRefExt: TActorRef {
    fn cast<M>(&self, message: M, sender: Option<ActorRef>)
        where
            M: Message,
    {
        self.tell(DynamicMessage::user(message), sender);
    }
    fn cast_async<M>(&self, message: M, sender: Option<ActorRef>)
        where
            M: AsyncMessage,
    {
        self.tell(DynamicMessage::async_user(message), sender);
    }
    fn cast_system<M>(&self, message: M, sender: Option<ActorRef>)
        where
            M: SystemMessage,
    {
        self.tell(DynamicMessage::system(message), sender);
    }
    fn resp<M>(&self, message: M)
        where
            M: DeferredMessage,
    {
        self.tell(DynamicMessage::deferred(message), ActorRef::no_sender());
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SerializedActorRef {
    pub path: String,
}

impl SerializedActorRef {
    pub fn parse_to_path(&self) -> anyhow::Result<ActorPath> {
        self.path.parse()
    }
}

impl Display for SerializedActorRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.path)
    }
}

impl Into<SerializedActorRef> for ActorRef {
    fn into(self) -> SerializedActorRef {
        SerializedActorRef {
            path: self.path().to_serialization(),
        }
    }
}

pub(crate) trait Cell {
    fn underlying(&self) -> ActorCell;
    fn children(&self) -> &RwLock<BTreeMap<String, ActorRef>>;
    fn get_single_child(&self, name: &String) -> Option<ActorRef>;
}

#[cfg(test)]
mod ref_test {
    #[test]
    fn test_serde() {
        // let actor_ref = SerializedActorRef {
        //     address: Address {
        //         protocol: "tcp".to_string(),
        //         system: "game".to_string(),
        //         addr: "127.0.0.1:1122".parse().unwrap(),
        //     },
        //     path: vec!["a".to_string(), "b".to_string()],
        //     uid: 112132434,
        // };
        // let url = actor_ref.to_string();
        // let de_actor_ref: SerializedActorRef = url.parse().unwrap();
        // assert_eq!(actor_ref, de_actor_ref);
    }
}

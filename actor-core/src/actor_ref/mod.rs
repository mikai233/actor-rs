use std::any::Any;
use std::cmp::Ordering;
use std::fmt::{Debug, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::iter::Peekable;
use std::ops::Deref;
use std::sync::Arc;

use bincode::de::Decoder;
use bincode::enc::Encoder;
use bincode::error::{DecodeError, EncodeError};
use bincode::{impl_borrow_decode, Decode, Encode};
use serde::de::{Error, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use tokio::task_local;

use crate::actor::actor_system::WeakActorSystem;
use crate::actor_path::ActorPath;
use crate::actor_path::TActorPath;
use crate::actor_ref::local_ref::LocalActorRef;
use crate::ext::as_any::AsAny;
use crate::message::{DynMessage, Message};
use crate::provider::ActorRefProvider;

pub mod actor_ref_factory;
pub(crate) mod dead_letter_ref;
pub mod deferred_ref;
pub(crate) mod empty_local_ref;
pub(crate) mod function_ref;
pub(crate) mod ignore_ref;
pub mod local_ref;
pub(crate) mod virtual_path_container;

task_local! {
    pub static PROVIDER: Arc<ActorRefProvider>;
}

pub trait TActorRef: Debug + Send + Sync + Any + AsAny {
    fn system(&self) -> &WeakActorSystem;

    fn path(&self) -> &ActorPath;

    fn tell(&self, message: DynMessage, sender: Option<ActorRef>);

    fn stop(&self);

    fn parent(&self) -> Option<&ActorRef>;

    fn get_child(&self, names: &mut Peekable<&mut dyn Iterator<Item = &str>>) -> Option<ActorRef>;

    fn resume(&self) {}

    fn suspend(&self) {}
}

impl<T: ?Sized> ActorRefExt for T where T: TActorRef {}

pub trait ActorRefExt: TActorRef {
    fn cast<M>(&self, message: M, sender: Option<ActorRef>)
    where
        M: Message,
    {
        self.tell(Box::new(message), sender);
    }

    fn cast_ns<M>(&self, message: M)
    where
        M: Message,
    {
        self.tell(Box::new(message), ActorRef::no_sender());
    }
}

#[derive(Clone)]
pub struct ActorRef(Arc<dyn TActorRef>);

impl ActorRef {
    pub fn new<R>(actor_ref: R) -> Self
    where
        R: TActorRef,
    {
        Self(Arc::new(actor_ref))
    }
}

impl ActorRef {
    pub fn no_sender() -> Option<ActorRef> {
        None
    }

    pub(crate) fn local(&self) -> Option<&LocalActorRef> {
        self.as_any().downcast_ref()
    }
}

impl Deref for ActorRef {
    type Target = Arc<dyn TActorRef>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
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

impl Hash for ActorRef {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.path().hash(state);
    }
}

impl PartialEq<Self> for ActorRef {
    fn eq(&self, other: &Self) -> bool {
        self.path().eq(other.path())
    }
}

impl Eq for ActorRef {}

impl PartialOrd<Self> for ActorRef {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.path().partial_cmp(other.path())
    }
}

impl Ord for ActorRef {
    fn cmp(&self, other: &Self) -> Ordering {
        self.path().cmp(other.path())
    }
}

impl Debug for ActorRef {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_struct("ActorRef")
            .field("path", self.path())
            .finish_non_exhaustive()
    }
}

impl Encode for ActorRef {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        let path = self.path().to_serialization_format();
        Encode::encode(&path, encoder)
    }
}

impl Decode for ActorRef {
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
        let path: String = Decode::decode(decoder)?;
        let actor_ref = PROVIDER
            .try_with(|provider| provider.resolve_actor_ref(&path))
            .map_err(|_| {
                DecodeError::Other("task local value PROVIDER not set in current decode scope")
            })?;
        Ok(actor_ref)
    }
}

impl_borrow_decode!(ActorRef);

struct ActorVisitor;

impl<'de> Visitor<'de> for ActorVisitor {
    type Value = ActorRef;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        write!(formatter, "serialization format actor path")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        let actor_ref = PROVIDER
            .try_with(|provider| provider.resolve_actor_ref(v))
            .map_err(|_| {
                Error::custom("task local value PROVIDER not set in current decode scope")
            })?;
        Ok(actor_ref)
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: Error,
    {
        let actor_ref = PROVIDER
            .try_with(|provider| provider.resolve_actor_ref(&v))
            .map_err(|_| {
                Error::custom("task local value PROVIDER not set in current decode scope")
            })?;
        Ok(actor_ref)
    }
}

impl Serialize for ActorRef {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let path = self.path().to_serialization_format();
        serializer.serialize_str(&path)
    }
}

impl<'de> Deserialize<'de> for ActorRef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(ActorVisitor)
    }
}

pub(crate) fn get_child_default(
    actor_ref: impl Into<ActorRef>,
    names: &mut Peekable<&mut dyn Iterator<Item = &str>>,
) -> Option<ActorRef> {
    match names.next() {
        None => Some(actor_ref.into()),
        Some(_) => None,
    }
}

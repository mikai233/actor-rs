use std::any::Any;
use std::cmp::Ordering;
use std::fmt::{Debug, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::sync::Arc;

use crate::{AsyncMessage, DynMessage, Message, SystemMessage, UntypedMessage};
use crate::actor::actor_path::{ActorPath, TActorPath};
use crate::actor::actor_system::ActorSystem;
use crate::actor::local_ref::LocalActorRef;
use crate::ext::as_any::AsAny;

pub trait TActorRef: Debug + Send + Sync + Any + AsAny {
    fn system(&self) -> ActorSystem;
    fn path(&self) -> &ActorPath;
    fn tell(&self, message: DynMessage, sender: Option<ActorRef>);
    fn stop(&self);
    fn parent(&self) -> Option<&ActorRef>;
    fn get_child(&self, names: Vec<String>) -> Option<ActorRef>;
}

impl<T: ?Sized> ActorRefExt for T where T: TActorRef {}

pub trait ActorRefExt: TActorRef {
    fn cast<M>(&self, message: M, sender: Option<ActorRef>)
        where
            M: Message,
    {
        self.tell(DynMessage::user(message), sender);
    }
    fn cast_async<M>(&self, message: M, sender: Option<ActorRef>)
        where
            M: AsyncMessage,
    {
        self.tell(DynMessage::async_user(message), sender);
    }
    fn resp<M>(&self, message: M)
        where
            M: UntypedMessage,
    {
        self.tell(DynMessage::untyped(message), ActorRef::no_sender());
    }
}

impl<T: ?Sized> ActorRefSystemExt for T where T: TActorRef {}

pub trait ActorRefSystemExt: TActorRef {
    fn cast_system<M>(&self, message: M, sender: Option<ActorRef>)
        where
            M: SystemMessage,
    {
        self.tell(DynMessage::system(message), sender);
    }
}

#[derive(Clone)]
pub struct ActorRef {
    inner: Arc<Box<dyn TActorRef>>,
}

impl ActorRef {
    pub fn new<R>(actor_ref: R) -> Self where R: TActorRef {
        Self {
            inner: Arc::new(Box::new(actor_ref))
        }
    }
}

impl ActorRef {
    pub fn no_sender() -> Option<ActorRef> {
        None
    }

    pub(crate) fn local(&self) -> Option<&LocalActorRef> {
        let actor_ref = &****self;
        actor_ref.as_any_ref().downcast_ref::<LocalActorRef>()
    }
}

impl Deref for ActorRef {
    type Target = Arc<Box<dyn TActorRef>>;

    fn deref(&self) -> &Self::Target {
        &self.inner
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
use std::any::Any;
use std::fmt::{Debug, Display, Formatter};
use std::hash::Hash;
use std::ops::Deref;
use std::sync::Arc;

use crate::{AsyncMessage, DynMessage, Message, SystemMessage, UntypedMessage};
use crate::actor::actor_path::{ActorPath, TActorPath};
use crate::actor_ref::local_ref::LocalActorRef;
use crate::ext::as_any::AsAny;
use crate::system::ActorSystem;

pub trait TActorRef: Debug + Send + Sync + Any + AsAny + Ord + PartialOrd + Eq + PartialOrd + Hash {
    fn system(&self) -> ActorSystem;
    fn path(&self) -> &ActorPath;
    fn tell(&self, message: DynMessage, sender: Option<ActorRef>);
    fn stop(&self);
    fn parent(&self) -> Option<&ActorRef>;
    fn get_child<I>(&self, names: I) -> Option<ActorRef>
        where
            I: IntoIterator<Item=String>;
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

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct ActorRef {
    inner: Arc<Box<dyn TActorRef>>,
}

impl Deref for ActorRef {
    type Target = Arc<Box<dyn TActorRef>>;

    fn deref(&self) -> &Self::Target {
        &self.inner
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
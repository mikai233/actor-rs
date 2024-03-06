use std::fmt::{Debug, Formatter};
use std::iter::Peekable;
use std::ops::Deref;
use std::sync::Arc;

use actor_derive::AsAny;

use crate::actor::actor_system::ActorSystem;
use crate::actor_path::ActorPath;
use crate::actor_ref::{ActorRef, TActorRef};
use crate::DynMessage;

#[derive(Clone, AsAny)]
pub struct FunctionRef {
    pub(crate) inner: Arc<Inner>,
}

pub struct Inner {
    pub(crate) system: ActorSystem,
    pub(crate) path: ActorPath,
    pub(crate) message_handler: Arc<Box<dyn Fn(DynMessage, Option<ActorRef>) + Send + Sync + 'static>>,
}

impl Deref for FunctionRef {
    type Target = Arc<Inner>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Debug for FunctionRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FunctionRef")
            .field("system", &"..")
            .field("path", &self.path)
            .field("message_handler", &"..")
            .finish()
    }
}

impl TActorRef for FunctionRef {
    fn system(&self) -> &ActorSystem {
        &self.system
    }

    fn path(&self) -> &ActorPath {
        &self.path
    }

    fn tell(&self, message: DynMessage, sender: Option<ActorRef>) {
        (self.message_handler)(message, sender);
    }

    fn stop(&self) {
        todo!()
    }

    fn parent(&self) -> Option<&ActorRef> {
        None
    }

    fn get_child(&self, names: &mut Peekable<&mut dyn Iterator<Item=&str>>) -> Option<ActorRef> {
        match names.next() {
            None => {
                Some(self.clone().into())
            }
            Some(_) => {
                None
            }
        }
    }
}

impl Into<ActorRef> for FunctionRef {
    fn into(self) -> ActorRef {
        ActorRef::new(self)
    }
}
use std::fmt::{Debug, Formatter};
use std::ops::Deref;
use std::sync::Arc;

use actor_derive::AsAny;

use crate::actor::actor_path::ActorPath;
use crate::actor::actor_ref::{ActorRef, get_child_default, TActorRef};
use crate::actor::actor_system::ActorSystem;
use crate::DynMessage;

#[derive(Clone, AsAny)]
pub struct EmptyLocalActorRef {
    pub(crate) inner: Arc<Inner>,
}

pub struct Inner {
    pub(crate) system: ActorSystem,
    pub(crate) path: ActorPath,
}

impl Debug for EmptyLocalActorRef {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_struct("EmptyLocalActorRef")
            .field("system", &"..")
            .field("path", &self.path)
            .finish()
    }
}

impl Deref for EmptyLocalActorRef {
    type Target = Arc<Inner>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl TActorRef for EmptyLocalActorRef {
    fn system(&self) -> &ActorSystem {
        &self.system
    }

    fn path(&self) -> &ActorPath {
        todo!()
    }

    fn tell(&self, _message: DynMessage, _sender: Option<ActorRef>) {
        todo!()
    }

    fn stop(&self) {}

    fn parent(&self) -> Option<&ActorRef> {
        None
    }

    fn get_child(&self, names: Box<dyn Iterator<Item=String>>) -> Option<ActorRef> {
        get_child_default(self.clone(), names)
    }
}

impl Into<ActorRef> for EmptyLocalActorRef {
    fn into(self) -> ActorRef {
        ActorRef::new(self)
    }
}
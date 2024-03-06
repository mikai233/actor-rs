use std::fmt::{Debug, Formatter};
use std::iter::Peekable;
use std::ops::Deref;
use std::sync::Arc;

use tracing::info;

use actor_derive::AsAny;

use crate::actor::actor_system::ActorSystem;
use crate::actor_path::ActorPath;
use crate::actor_ref::{ActorRef, TActorRef};
use crate::DynMessage;

#[derive(Clone, AsAny)]
pub struct DeadLetterActorRef {
    pub(crate) inner: Arc<Inner>,
}

pub struct Inner {
    pub(crate) system: ActorSystem,
    pub(crate) path: ActorPath,
}

impl DeadLetterActorRef {
    pub(crate) fn new(system: ActorSystem, path: ActorPath) -> Self {
        Self {
            inner: Arc::new(Inner { system, path }),
        }
    }
}

impl Deref for DeadLetterActorRef {
    type Target = Arc<Inner>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Debug for DeadLetterActorRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeadLetterActorRef")
            .field("system", &"..")
            .field("path", &self.path)
            .finish()
    }
}

impl TActorRef for DeadLetterActorRef {
    fn system(&self) -> &ActorSystem {
        &self.system
    }

    fn path(&self) -> &ActorPath {
        &self.path
    }

    fn tell(&self, message: DynMessage, sender: Option<ActorRef>) {
        let name = message.name();
        match sender {
            None => {
                info!("dead letter recv message {}", name);
            }
            Some(sender) => {
                info!("dead letter recv message {} from {}", name, sender);
            }
        }
    }

    fn stop(&self) {}

    fn parent(&self) -> Option<&ActorRef> {
        None
    }

    fn get_child(&self, _names: &mut Peekable<&mut dyn Iterator<Item=&str>>) -> Option<ActorRef> {
        None
    }
}

impl Into<ActorRef> for DeadLetterActorRef {
    fn into(self) -> ActorRef {
        ActorRef::new(self)
    }
}
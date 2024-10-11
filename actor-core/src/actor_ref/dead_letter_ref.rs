use std::fmt::{Debug, Formatter};
use std::iter::Peekable;
use std::sync::Arc;

use tracing::info;

use actor_derive::AsAny;

use crate::actor_path::ActorPath;
use crate::actor_ref::{ActorRef, TActorRef};
use crate::message::DynMessage;

#[derive(Clone, AsAny, derive_more::Deref)]
pub struct DeadLetterActorRef(Arc<DeadLetterActorRefInner>);

pub struct DeadLetterActorRefInner {
    pub(crate) path: ActorPath,
}

impl DeadLetterActorRef {
    pub(crate) fn new(path: ActorPath) -> Self {
        let inner = DeadLetterActorRefInner { path };
        DeadLetterActorRef(inner.into())
    }
}

impl Debug for DeadLetterActorRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeadLetterActorRef")
            .field("path", &self.path)
            .finish()
    }
}

impl TActorRef for DeadLetterActorRef {
    fn path(&self) -> &ActorPath {
        &self.path
    }

    fn tell(&self, message: DynMessage, sender: Option<ActorRef>) {
        let name = message.signature();
        match sender {
            None => {
                info!("dead letter recv message {}", name);
            }
            Some(sender) => {
                info!("dead letter recv message {} from {}", name, sender);
            }
        }
    }

    fn start(&self) {}

    fn stop(&self) {}

    fn resume(&self) {}

    fn suspend(&self) {}

    fn parent(&self) -> Option<&dyn TActorRef> {
        None
    }

    fn get_child(&self, _names: &mut Peekable<&mut dyn Iterator<Item = &str>>) -> Option<ActorRef> {
        None
    }
}

impl Into<ActorRef> for DeadLetterActorRef {
    fn into(self) -> ActorRef {
        ActorRef::new(self)
    }
}

use std::fmt::{Debug, Formatter};
use std::ops::Deref;
use std::sync::Arc;

use tracing::info;

use crate::actor::actor_path::ActorPath;
use crate::actor::actor_ref::{ActorRef, TActorRef};
use crate::DynMessage;
use crate::system::ActorSystem;

#[derive(Clone)]
pub struct DeadLetterActorRef {
    pub(crate) inner: Arc<Inner>,
}

pub struct Inner {
    pub(crate) system: ActorSystem,
    pub(crate) path: ActorPath,
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
    fn system(&self) -> ActorSystem {
        self.system.clone()
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

    fn get_child<I>(&self, _names: I) -> Option<ActorRef> where I: IntoIterator<Item=String> {
        None
    }
}

use std::fmt::{Debug, Formatter};
use std::sync::Arc;

use crate::actor_path::ActorPath;
use crate::actor_ref::{ActorRef, TActorRef};
use crate::DynamicMessage;
use crate::system::ActorSystem;

#[derive(Clone)]
pub struct FunctionRef {
    pub(crate) system: ActorSystem,
    pub(crate) path: ActorPath,
    pub(crate) message_handler: Arc<Box<dyn Fn(DynamicMessage, Option<ActorRef>) + Send + Sync + 'static>>,
}

impl Debug for FunctionRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FunctionRef")
            .field("system", &self.system)
            .field("path", &self.path)
            .field("message_handler", &"..")
            .finish()
    }
}

impl TActorRef for FunctionRef where {
    fn system(&self) -> ActorSystem {
        self.system.clone()
    }

    fn path(&self) -> &ActorPath {
        &self.path
    }

    fn tell(&self, message: DynamicMessage, sender: Option<ActorRef>) {
        (self.message_handler)(message, sender);
    }

    fn stop(&self) {
        todo!()
    }

    fn parent(&self) -> Option<&ActorRef> {
        None
    }

    fn get_child<I>(&self, names: I) -> Option<ActorRef> where I: IntoIterator<Item=String> {
        let mut names = names.into_iter();
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
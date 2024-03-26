use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::ops::{Deref, DerefMut};

use crate::actor_ref::ActorRef;
use crate::DynMessage;
use crate::message::terminated::Terminated;

#[derive(Default)]
pub(super) struct Watching(HashMap<ActorRef, Box<dyn FnOnce(Terminated) -> DynMessage + Send>>);

impl Deref for Watching {
    type Target = HashMap<ActorRef, Box<dyn FnOnce(Terminated) -> DynMessage + Send>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Watching {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Debug for Watching {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let watching = self.keys()
            .map(|key| (key, ".."))
            .collect::<HashMap<_, _>>();
        Debug::fmt(&watching, f)
    }
}
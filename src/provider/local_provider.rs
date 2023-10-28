use std::sync::{Arc, RwLock};

use crate::actor::Actor;
use crate::actor_path::ActorPath;
use crate::actor_ref::{ActorRef, TActorRef};
use crate::actor_ref::local_ref::LocalActorRef;
use crate::cell::ActorCell;
use crate::props::Props;
use crate::provider::{ActorRefFactory, TActorRefProvider};

#[derive(Debug, Clone)]
pub(crate) struct LocalActorRefProvider {
    guardian: ActorCell,
}

impl TActorRefProvider for LocalActorRefProvider {
    fn resolve_actor_ref(&self, path: String) -> ActorRef {
        todo!()
    }

    fn resolve_actor_ref_of_path(&self, path: ActorPath) -> ActorRef {
        todo!()
    }
}
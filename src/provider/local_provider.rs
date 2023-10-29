use crate::actor_path::ActorPath;
use crate::actor_ref::ActorRef;
use crate::cell::ActorCell;
use crate::provider::{ActorRefFactory, TActorRefProvider};

#[derive(Debug, Clone)]
pub struct LocalActorRefProvider {
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
use std::sync::{Arc, RwLock};
use crate::actor::Actor;
use crate::actor_path::ActorPath;
use crate::actor_ref::ActorRef;
use crate::actor_ref::local_ref::LocalActorRef;
use crate::props::Props;
use crate::provider::local_provider::LocalActorRefProvider;
use crate::provider::TActorRefProvider;

#[derive(Debug, Clone)]
pub(crate) struct RemoteActorRefProvider {
    local: LocalActorRefProvider,
}

impl TActorRefProvider for RemoteActorRefProvider {
    fn resolve_actor_ref(&self, path: String) -> ActorRef {
        todo!()
    }
    fn resolve_actor_ref_of_path(&self, path: ActorPath) -> ActorRef {
        todo!()
    }
}
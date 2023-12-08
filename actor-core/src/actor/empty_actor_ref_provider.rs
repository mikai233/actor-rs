use std::fmt::{Debug, Formatter};
use crate::actor::actor_path::ActorPath;
use crate::actor::actor_ref::ActorRef;
use crate::actor::actor_ref_provider::ActorRefProvider;
use crate::actor::local_ref::LocalActorRef;
use crate::props::Props;

#[derive(Debug, Default)]
pub(crate) struct EmptyActorRefProvider;

impl ActorRefProvider for EmptyActorRefProvider {
    fn root_guardian(&self) -> &LocalActorRef {
        unimplemented!()
    }

    fn guardian(&self) -> &LocalActorRef {
        unimplemented!()
    }

    fn system_guardian(&self) -> &LocalActorRef {
        unimplemented!()
    }

    fn root_path(&self) -> &ActorPath {
        unimplemented!()
    }

    fn temp_path(&self) -> ActorPath {
        unimplemented!()
    }

    fn temp_path_of_prefix(&self, prefix: Option<String>) -> ActorPath {
        unimplemented!()
    }

    fn temp_container(&self) -> ActorRef {
        unimplemented!()
    }

    fn register_temp_actor(&self, actor: ActorRef, path: &ActorPath) {
        unimplemented!()
    }

    fn unregister_temp_actor(&self, path: &ActorPath) {
        unimplemented!()
    }

    fn actor_of(&self, props: Props, supervisor: &ActorRef) -> anyhow::Result<ActorRef> {
        unimplemented!()
    }

    fn resolve_actor_ref_of_path(&self, path: &ActorPath) -> ActorRef {
        unimplemented!()
    }

    fn dead_letters(&self) -> &ActorRef {
        unimplemented!()
    }
}
use crate::actor_path::ActorPath;
use crate::actor_ref::ActorRef;
use crate::actor_ref::local_ref::LocalActorRef;
use crate::props::Props;
use crate::provider::TActorRefProvider;

#[derive(Debug, Default)]
pub struct EmptyActorRefProvider;

impl TActorRefProvider for EmptyActorRefProvider {
    fn root_guardian(&self) -> &LocalActorRef {
        panic!("unreachable");
    }

    fn guardian(&self) -> &LocalActorRef {
        panic!("unreachable");
    }

    fn system_guardian(&self) -> &LocalActorRef {
        panic!("unreachable");
    }

    fn root_path(&self) -> &ActorPath {
        panic!("unreachable");
    }

    fn temp_path(&self) -> ActorPath {
        panic!("unreachable");
    }

    fn temp_path_of_prefix(&self, _prefix: Option<String>) -> ActorPath {
        panic!("unreachable");
    }

    fn temp_container(&self) -> ActorRef {
        panic!("unreachable");
    }

    fn register_temp_actor(&self, _actor: ActorRef, _path: &ActorPath) {
        panic!("unreachable");
    }

    fn unregister_temp_actor(&self, _path: &ActorPath) {
        panic!("unreachable");
    }

    fn actor_of(&self, _props: Props, _supervisor: &ActorRef) -> anyhow::Result<ActorRef> {
        panic!("unreachable");
    }

    fn resolve_actor_ref_of_path(&self, _path: &ActorPath) -> ActorRef {
        panic!("unreachable");
    }

    fn dead_letters(&self) -> &ActorRef {
        panic!("unreachable");
    }
}
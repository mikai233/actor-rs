use crate::actor::Actor;
use crate::actor_path::ActorPath;
use crate::actor_ref::ActorRef;
use crate::actor_ref::local_ref::LocalActorRef;
use crate::props::Props;
use crate::provider::TActorRefProvider;

#[derive(Debug, Clone)]
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

    fn temp_path(&self) -> &ActorPath {
        panic!("unreachable");
    }

    fn temp_path_of(&self) -> &ActorPath {
        panic!("unreachable");
    }

    fn register_temp_actor(&self, actor: ActorRef, path: ActorPath) {
        panic!("unreachable");
    }

    fn unregister_temp_actor(&self, path: ActorPath) {
        panic!("unreachable");
    }

    fn actor_of<T>(&self, actor: T, arg: T::A, props: Props, supervisor: &ActorRef, path: ActorPath) -> anyhow::Result<ActorRef> where T: Actor {
        panic!("unreachable");
    }

    fn resolve_actor_ref(&self, path: &String) -> ActorRef {
        panic!("unreachable");
    }

    fn resolve_actor_ref_of_path(&self, path: &ActorPath) -> ActorRef {
        panic!("unreachable");
    }

    fn dead_letters(&self) -> &ActorRef {
        panic!("unreachable");
    }
}
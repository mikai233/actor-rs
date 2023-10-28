use enum_dispatch::enum_dispatch;
use crate::actor::Actor;

use crate::actor_path::ActorPath;
use crate::actor_ref::ActorRef;
use crate::actor_ref::local_ref::LocalActorRef;
use crate::props::Props;
use crate::provider::local_provider::LocalActorRefProvider;
use crate::provider::remote_provider::RemoteActorRefProvider;
use crate::system::ActorSystem;

mod remote_provider;
pub(crate) mod local_provider;


#[enum_dispatch]
#[derive(Debug, Clone)]
pub(crate) enum ActorRefProvider {
    LocalActorRefProvider,
    RemoteActorRefProvider,
}

#[enum_dispatch(ActorRefProvider)]
pub(crate) trait TActorRefProvider: Send {
    fn resolve_actor_ref(&self, path: String) -> ActorRef;
    fn resolve_actor_ref_of_path(&self, path: ActorPath) -> ActorRef;
}

pub trait ActorRefFactory {
    fn system(&self) -> &ActorSystem;
    fn provider(&self) -> &ActorRefProvider;
    fn guardian(&self) -> &ActorRef;
    fn lookup_root(&self) -> &ActorRef;
    fn actor_of<T>(&self, actor: T, arg: T::A, props: Props, name: Option<String>) -> ActorRef where T: Actor;
    fn actor_selection(&self, path: String) {}
    fn actor_selection_of_path(&self, path: ActorPath) {}
    fn stop(&self, actor: &ActorRef);
}
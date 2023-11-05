use enum_dispatch::enum_dispatch;

use crate::actor::Actor;
use crate::actor_path::ActorPath;
use crate::actor_ref::local_ref::LocalActorRef;
use crate::actor_ref::ActorRef;
use crate::props::Props;
use crate::provider::empty_provider::EmptyActorRefProvider;
use crate::provider::local_provider::LocalActorRefProvider;
use crate::provider::remote_provider::RemoteActorRefProvider;
use crate::system::ActorSystem;

pub(crate) mod empty_provider;
pub(crate) mod local_provider;
pub(crate) mod remote_provider;

#[enum_dispatch]
#[derive(Debug, Clone)]
pub enum ActorRefProvider {
    EmptyActorRefProvider,
    LocalActorRefProvider,
    RemoteActorRefProvider,
}

impl ActorRefProvider {
    fn remote_or_panic(&self) -> &RemoteActorRefProvider {
        match self {
            ActorRefProvider::RemoteActorRefProvider(r) => r,
            _ => panic!("expect RemoteActorRefProvider"),
        }
    }
}

#[enum_dispatch(ActorRefProvider)]
pub trait TActorRefProvider: Send {
    fn root_guardian(&self) -> &LocalActorRef;
    fn guardian(&self) -> &LocalActorRef;
    fn system_guardian(&self) -> &LocalActorRef;
    fn root_path(&self) -> &ActorPath;
    fn temp_path(&self) -> &ActorPath;
    fn temp_path_of(&self) -> &ActorPath;
    fn register_temp_actor(&self, actor: ActorRef, path: ActorPath);
    fn unregister_temp_actor(&self, path: ActorPath);
    fn actor_of<T>(
        &self,
        actor: T,
        arg: T::A,
        props: Props,
        supervisor: &ActorRef,
        path: ActorPath,
    ) -> anyhow::Result<ActorRef>
        where
            T: Actor;
    fn resolve_actor_ref(&self, path: &String) -> ActorRef;
    fn resolve_actor_ref_of_path(&self, path: &ActorPath) -> ActorRef;
    fn dead_letters(&self) -> &ActorRef;
}

pub trait ActorRefFactory {
    fn system(&self) -> &ActorSystem;
    fn provider(&self) -> ActorRefProvider;
    fn guardian(&self) -> LocalActorRef;
    fn lookup_root(&self) -> ActorRef;
    fn actor_of<T>(
        &self,
        actor: T,
        arg: T::A,
        props: Props,
        name: Option<String>,
    ) -> anyhow::Result<ActorRef>
        where
            T: Actor;
    fn actor_selection(&self, path: String) {}
    fn actor_selection_of_path(&self, path: ActorPath) {}
    fn stop(&self, actor: &ActorRef);
}

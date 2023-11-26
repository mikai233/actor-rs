use std::sync::Arc;

use enum_dispatch::enum_dispatch;

use crate::Actor;
use crate::actor_path::ActorPath;
use crate::actor_ref::ActorRef;
use crate::actor_ref::local_ref::LocalActorRef;
use crate::props::Props;
use crate::provider::empty_provider::EmptyActorRefProvider;
use crate::provider::local_provider::LocalActorRefProvider;
use crate::provider::remote_provider::RemoteActorRefProvider;
use crate::system::ActorSystem;

pub(crate) mod empty_provider;
pub(crate) mod local_provider;
pub(crate) mod remote_provider;

#[enum_dispatch]
#[derive(Debug)]
pub enum ActorRefProvider {
    EmptyActorRefProvider,
    LocalActorRefProvider,
    RemoteActorRefProvider,
}

impl ActorRefProvider {
    pub(crate) fn remote_or_panic(&self) -> &RemoteActorRefProvider {
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
    fn temp_path(&self) -> ActorPath;
    fn temp_path_of_prefix(&self, prefix: Option<String>) -> ActorPath;
    fn temp_container(&self) -> ActorRef;
    fn register_temp_actor(&self, actor: ActorRef, path: &ActorPath);
    fn unregister_temp_actor(&self, path: &ActorPath);
    fn actor_of<T>(
        &self,
        actor: T,
        arg: T::A,
        props: Props,
        supervisor: &ActorRef,
    ) -> anyhow::Result<ActorRef>
        where
            T: Actor;
    fn resolve_actor_ref(&self, path: impl AsRef<str>) -> ActorRef {
        match path.as_ref().parse::<ActorPath>() {
            Ok(actor_path) => self.resolve_actor_ref_of_path(&actor_path),
            Err(_) => self.dead_letters().clone(),
        }
    }
    fn resolve_actor_ref_of_path(&self, path: &ActorPath) -> ActorRef;
    fn dead_letters(&self) -> &ActorRef;
}

pub trait ActorRefFactory {
    fn system(&self) -> &ActorSystem;
    fn provider(&self) -> Arc<ActorRefProvider>;
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
    fn actor_selection(&self, _path: String) {
        todo!()
    }
    fn actor_selection_of_path(&self, _path: ActorPath) {
        todo!()
    }
    fn stop(&self, actor: &ActorRef);
}

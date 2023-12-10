use std::any::Any;
use std::fmt::Debug;
use std::ops::Deref;

use crate::actor::actor_path::ActorPath;
use crate::actor::actor_path::TActorPath;
use crate::actor::actor_ref::ActorRef;
use crate::actor::address::Address;
use crate::actor::local_ref::LocalActorRef;
use crate::actor::props::Props;
use crate::ext::as_any::AsAny;

pub trait TActorRefProvider: Send + Sync + Any + AsAny + Debug {
    fn root_guardian(&self) -> &LocalActorRef;
    fn root_guardian_at(&self, address: &Address) -> ActorRef;
    fn guardian(&self) -> &LocalActorRef;
    fn system_guardian(&self) -> &LocalActorRef;
    fn root_path(&self) -> &ActorPath;
    fn temp_path(&self) -> ActorPath;
    fn temp_path_of_prefix(&self, prefix: Option<String>) -> ActorPath;
    fn temp_container(&self) -> ActorRef;
    fn register_temp_actor(&self, actor: ActorRef, path: &ActorPath);
    fn unregister_temp_actor(&self, path: &ActorPath);
    fn actor_of(&self, props: Props, supervisor: &ActorRef) -> anyhow::Result<ActorRef>;
    fn resolve_actor_ref(&self, path: &str) -> ActorRef {
        match path.parse::<ActorPath>() {
            Ok(actor_path) => self.resolve_actor_ref_of_path(&actor_path),
            Err(_) => self.dead_letters().clone(),
        }
    }
    fn resolve_actor_ref_of_path(&self, path: &ActorPath) -> ActorRef;
    fn dead_letters(&self) -> &ActorRef;
    fn get_default_address(&self) -> Address {
        self.root_path().address()
    }
}

#[derive(Debug)]
pub struct ActorRefProvider {
    inner: Box<dyn TActorRefProvider>,
}

impl Deref for ActorRefProvider {
    type Target = Box<dyn TActorRefProvider>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl ActorRefProvider {
    pub fn new<P>(provider: P) -> Self where P: TActorRefProvider {
        Self {
            inner: Box::new(provider),
        }
    }
}
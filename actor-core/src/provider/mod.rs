use std::any::Any;
use std::any::type_name;
use std::fmt::Debug;
use std::ops::Deref;

use tokio::sync::broadcast::Receiver;

use crate::actor::address::Address;
use crate::actor::props::Props;
use crate::actor_path::ActorPath;
use crate::actor_path::TActorPath;
use crate::actor_ref::ActorRef;
use crate::actor_ref::local_ref::LocalActorRef;
use crate::ext::as_any::AsAny;

pub mod local_actor_ref_provider;
pub mod empty_actor_ref_provider;

pub trait TActorRefProvider: Send + Sync + Any + AsAny + Debug {
    fn root_guardian(&self) -> &LocalActorRef;

    fn root_guardian_at(&self, address: &Address) -> ActorRef;

    fn guardian(&self) -> &LocalActorRef;

    fn system_guardian(&self) -> &LocalActorRef;

    fn root_path(&self) -> &ActorPath;

    fn temp_path(&self) -> ActorPath;

    fn temp_path_of_prefix(&self, prefix: Option<&String>) -> ActorPath;

    fn temp_container(&self) -> ActorRef;

    fn register_temp_actor(&self, actor: ActorRef, path: &ActorPath);

    fn unregister_temp_actor(&self, path: &ActorPath);

    fn spawn_actor(&self, props: Props, supervisor: &ActorRef) -> anyhow::Result<ActorRef>;

    fn resolve_actor_ref(&self, path: &str) -> ActorRef {
        match path.parse::<ActorPath>() {
            Ok(actor_path) => self.resolve_actor_ref_of_path(&actor_path),
            Err(_) => self.dead_letters().clone(),
        }
    }

    fn resolve_actor_ref_of_path(&self, path: &ActorPath) -> ActorRef;

    fn dead_letters(&self) -> &ActorRef;

    fn get_default_address(&self) -> &Address {
        self.root_path().address()
    }

    fn termination_rx(&self) -> Receiver<()>;

    fn as_provider(&self, name: &str) -> Option<&dyn TActorRefProvider>;
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

pub fn downcast_provider<P>(provider: &ActorRefProvider) -> &P where P: TActorRefProvider {
    let provider_name = type_name::<P>();
    provider.as_provider(provider_name)
        .expect(&format!("{} not found", provider_name))
        .as_any()
        .downcast_ref::<P>()
        .expect(&format!("cannot downcast provider to {}", provider_name))
}

fn cast_self_to_dyn<'a, P>(name: &str, provider: &'a P) -> Option<&'a dyn TActorRefProvider> where P: TActorRefProvider {
    if name == type_name::<P>() {
        Some(provider)
    } else {
        None
    }
}
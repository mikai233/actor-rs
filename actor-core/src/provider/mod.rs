use crate::actor::actor_system::Settings;
use crate::actor::address::Address;
use crate::actor::props::Props;
use crate::actor_path::ActorPath;
use crate::actor_path::TActorPath;
use crate::actor_ref::local_ref::LocalActorRef;
use crate::actor_ref::{ActorRef, TActorRef};
use crate::ext::as_any::AsAny;
use std::any::type_name;
use std::any::Any;
use std::fmt::Debug;
use std::sync::Arc;
use tokio::sync::broadcast::Receiver;

pub mod local_provider;
pub mod provider;

pub trait TActorRefProvider: Send + Sync + Any + AsAny + Debug {
    fn settings(&self) -> &Settings;

    fn root_guardian(&self) -> &LocalActorRef;

    fn root_guardian_at(&self, address: &Address) -> ActorRef;

    fn guardian(&self) -> &LocalActorRef;

    fn system_guardian(&self) -> &LocalActorRef;

    fn root_path(&self) -> &ActorPath;

    fn temp_path(&self) -> ActorPath;

    fn temp_path_of_prefix(&self, prefix: Option<&str>) -> ActorPath;

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

    fn ignore_ref(&self) -> &ActorRef;

    fn get_default_address(&self) -> &Address {
        self.root_path().address()
    }

    fn termination_rx(&self) -> Receiver<()>;

    fn as_provider(&self, name: &str) -> Option<&dyn TActorRefProvider>;
}

#[derive(Debug, Clone, derive_more::Deref)]
pub struct ActorRefProvider(Arc<dyn TActorRefProvider>);

impl ActorRefProvider {
    pub fn new<P>(provider: P) -> Self
    where
        P: TActorRefProvider,
    {
        Self(Arc::new(provider))
    }

    pub fn downcast_ref<P>(&self) -> Option<&P>
    where
        P: TActorRefProvider,
    {
        self.0
            .as_provider(type_name::<P>())
            .and_then(|provider| provider.as_any().downcast_ref::<P>())
    }
}

impl AsRef<dyn TActorRefProvider> for ActorRefProvider {
    fn as_ref(&self) -> &dyn TActorRefProvider {
        self.0.as_ref()
    }
}

fn cast_self_to_dyn<'a, P>(name: &str, provider: &'a P) -> Option<&'a dyn TActorRefProvider>
where
    P: TActorRefProvider,
{
    if name == type_name::<P>() {
        Some(provider)
    } else {
        None
    }
}

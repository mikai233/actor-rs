use std::sync::Arc;

use tokio::runtime::Handle;

use crate::actor::actor_system::ActorSystem;
use crate::actor::props::DeferredSpawn;
use crate::config::ConfigBuilder;
use crate::config::core_config::CoreConfig;
use crate::provider::ActorRefProvider;
use crate::provider::local_actor_ref_provider::LocalActorRefProvider;

pub type ProviderBuilder = Box<dyn Fn(ActorSystem) -> eyre::Result<(ActorRefProvider, Vec<Box<dyn DeferredSpawn>>)>>;

#[derive(Clone)]
pub struct ActorSetting {
    pub provider: Arc<ProviderBuilder>,
    pub config: CoreConfig,
    pub handle: Option<Handle>,
}

impl ActorSetting {
    pub fn new<F>(provider: F, config: CoreConfig, handle: Option<Handle>) -> eyre::Result<Self>
        where
            F: Fn(ActorSystem) -> eyre::Result<(ActorRefProvider, Vec<Box<dyn DeferredSpawn>>)> + 'static
    {
        let setting = Self {
            provider: Arc::new(Box::new(provider)),
            config,
            handle,
        };
        Ok(setting)
    }


    pub fn new_with_default_config<F>(provider: F) -> eyre::Result<Self>
        where
            F: Fn(ActorSystem) -> eyre::Result<(ActorRefProvider, Vec<Box<dyn DeferredSpawn>>)> + 'static
    {
        let config = CoreConfig::builder().build()?;
        Self::new(provider, config, None)
    }
}

impl Default for ActorSetting {
    fn default() -> Self {
        let local_fn = |system: ActorSystem| {
            LocalActorRefProvider::new(system, None).map(|(r, d)| (r.into(), d))
        };
        Self {
            provider: Arc::new(Box::new(local_fn)),
            config: CoreConfig::builder().build().unwrap(),
            handle: None,
        }
    }
}
use std::sync::Arc;

use tokio::runtime::Handle;

use crate::actor::actor_system::ActorSystem;
use crate::actor::props::DeferredSpawn;
use crate::config::core_config::CoreConfig;
use crate::provider::ActorRefProvider;
use crate::provider::local_actor_ref_provider::LocalActorRefProvider;

#[derive(Clone)]
pub struct ActorSetting {
    pub provider_fn: Arc<Box<dyn Fn(&ActorSystem) -> eyre::Result<(ActorRefProvider, Vec<Box<dyn DeferredSpawn>>)>>>,
    pub config: CoreConfig,
    pub handle: Option<Handle>,
}

impl ActorSetting {
    pub fn builder() -> ActorSettingBuilder {
        ActorSettingBuilder::new()
    }
}

impl Default for ActorSetting {
    fn default() -> Self {
        let local_fn = |system: &ActorSystem| {
            LocalActorRefProvider::new(system.downgrade(), None).map(|(r, d)| (r.into(), d))
        };
        Self {
            provider_fn: Arc::new(Box::new(local_fn)),
            config: CoreConfig::default(),
            handle: None,
        }
    }
}

#[derive(Default)]
pub struct ActorSettingBuilder {
    pub provider_fn: Option<Box<dyn Fn(&ActorSystem) -> eyre::Result<(ActorRefProvider, Vec<Box<dyn DeferredSpawn>>)>>>,
    pub config: Option<CoreConfig>,
    pub handle: Option<Handle>,
}

impl ActorSettingBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn provider_fn<F>(mut self, provider_fn: F) -> Self where F: Fn(&ActorSystem) -> eyre::Result<(ActorRefProvider, Vec<Box<dyn DeferredSpawn>>)> + 'static {
        self.provider_fn = Some(Box::new(provider_fn));
        self
    }

    pub fn config(mut self, config: CoreConfig) -> Self {
        self.config = Some(config);
        self
    }

    pub fn handle(mut self, handle: Handle) -> Self {
        self.handle = Some(handle);
        self
    }

    pub fn build(self) -> ActorSetting {
        let Self { provider_fn: provider, config: core_config, handle } = self;
        let provider = provider.unwrap_or_else(|| {
            let local_fn = |system: &ActorSystem| {
                LocalActorRefProvider::new(system.downgrade(), None).map(|(r, d)| (r.into(), d))
            };
            Box::new(local_fn)
        });
        let core_config = core_config.unwrap_or_default();
        ActorSetting {
            provider_fn: provider.into(),
            config: core_config,
            handle,
        }
    }
}
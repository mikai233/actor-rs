use std::collections::HashMap;
use std::sync::Arc;

use crate::actor::actor_ref_provider::ActorRefProvider;
use crate::actor::actor_system::ActorSystem;
use crate::actor::config::Config;
use crate::actor::local_actor_ref_provider::LocalActorRefProvider;
use crate::actor::props::DeferredSpawn;

#[derive(Clone)]
pub struct ActorSetting {
    pub provider_fn: Arc<Box<dyn Fn(&ActorSystem) -> anyhow::Result<(ActorRefProvider, Vec<Box<dyn DeferredSpawn>>)>>>,
    pub config: HashMap<&'static str, Box<dyn Config>>,
}

impl Default for ActorSetting {
    fn default() -> Self {
        let local_fn = |system: &ActorSystem| {
            LocalActorRefProvider::new(system, None).map(|(r, d)| (r.into(), d))
        };
        Self {
            provider_fn: Arc::new(Box::new(local_fn)),
            config: HashMap::new(),
        }
    }
}

impl ActorSetting {
    pub fn with_provider<F>(&mut self, provider_fn: F) -> &mut Self where F: Fn(&ActorSystem) -> anyhow::Result<(ActorRefProvider, Vec<Box<dyn DeferredSpawn>>)> + 'static {
        self.provider_fn = Arc::new(Box::new(provider_fn));
        self
    }

    pub fn with_config<C>(&mut self, config: C) -> &mut Self where C: Config {
        self.config.insert(std::any::type_name::<C>(), Box::new(config));
        self
    }
}
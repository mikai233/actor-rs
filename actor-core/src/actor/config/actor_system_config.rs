use std::sync::Arc;

use crate::actor::actor_ref_provider::ActorRefProvider;
use crate::actor::actor_system::ActorSystem;
use crate::actor::local_actor_ref_provider::LocalActorRefProvider;
use crate::actor::props::DeferredSpawn;

#[derive(Clone)]
pub struct ActorSystemConfig {
    pub provider_fn: Arc<Box<dyn Fn(&ActorSystem) -> anyhow::Result<(ActorRefProvider, Vec<DeferredSpawn>)>>>,
}

impl Default for ActorSystemConfig {
    fn default() -> Self {
        Self {
            provider_fn: Arc::new(Box::new(|system| LocalActorRefProvider::new(system, None).map(|(r, d)| (r.into(), d)))),
        }
    }
}

impl ActorSystemConfig {
    pub fn with_provider<F>(&mut self, provider_fn: F) -> &mut Self where F: Fn(&ActorSystem) -> anyhow::Result<(ActorRefProvider, Vec<DeferredSpawn>)> + 'static {
        self.provider_fn = Arc::new(Box::new(provider_fn));
        self
    }
}
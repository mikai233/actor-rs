use config::Config;

use crate::actor::actor_system::ActorSystem;
use crate::actor::props::DeferredSpawn;
use crate::provider::TActorRefProvider;

pub trait ProviderBuilder<P>: Sized {
    fn build(system: ActorSystem, config: Config, registry: MessageRegistry) -> anyhow::Result<Provider<P>>;
}

pub struct Provider<P> {
    pub provider: P,
    pub spawns: Vec<Box<dyn DeferredSpawn>>,
}

impl<P> Provider<P>
where
    P: TActorRefProvider,
{
    pub fn new(provider: P, spawns: Vec<Box<dyn DeferredSpawn>>) -> Self {
        Self {
            provider,
            spawns,
        }
    }
}
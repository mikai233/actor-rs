use config::Config;

use crate::message::message_registry::MessageRegistry;
use crate::provider::builder::ProviderBuilder;
use crate::provider::TActorRefProvider;

#[derive(Debug, Clone)]
pub struct ActorSetting<B: ProviderBuilder<P>, P: TActorRefProvider> {
    pub config: Config,
    pub registry: MessageRegistry,
    phantom: std::marker::PhantomData<B>,
}

impl<B, P> ActorSetting<B, P>
where
    B: ProviderBuilder<P>,
    P: TActorRefProvider,
{
    pub fn new<F>(config: Config, registry: MessageRegistry) -> Self {
        Self {
            config,
            registry,
            phantom: std::marker::PhantomData,
        }
    }
}
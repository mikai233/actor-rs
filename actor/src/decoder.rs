use crate::DynamicMessage;
use crate::provider::ActorRefProvider;

pub trait MessageDecoder: Send + Sync + 'static {
    fn decode(&self, provider: &ActorRefProvider, bytes: &[u8]) -> anyhow::Result<DynamicMessage>;
}
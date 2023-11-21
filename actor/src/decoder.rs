use crate::actor::DynamicMessage;

pub trait MessageDecoder: Send + Sync + 'static {
    fn decode(&self, bytes: &[u8]) -> anyhow::Result<DynamicMessage>;
}
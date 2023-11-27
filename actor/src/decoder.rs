use dyn_clone::DynClone;

use crate::DynamicMessage;
use crate::provider::ActorRefProvider;

pub trait MessageDecoder: Send + Sync + DynClone + 'static {
    fn decode(&self, provider: &ActorRefProvider, bytes: &[u8]) -> anyhow::Result<DynamicMessage>;
}

dyn_clone::clone_trait_object!(MessageDecoder);
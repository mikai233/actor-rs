use std::sync::Arc;

use dyn_clone::DynClone;

use crate::actor::actor_ref_provider::ActorRefProvider;
use crate::DynMessage;

pub trait MessageDecoder: Send + Sync + DynClone + 'static {
    fn decode(&self, provider: &Arc<Box<dyn ActorRefProvider>>, bytes: &[u8]) -> anyhow::Result<DynMessage>;
}

dyn_clone::clone_trait_object!(MessageDecoder);
use actor_derive::AsAny;
use crate::actor::config::Config;

#[derive(Debug, Clone, AsAny)]
pub struct ActorConfig {}

impl Config for ActorConfig {
    fn with_fallback(&self, fallback: Box<dyn Config>) -> anyhow::Result<Box<dyn Config>> {
        todo!()
    }
}
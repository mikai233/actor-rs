use actor_core::message::message_registry::MessageRegistry;
use config::Config;

#[derive(Debug, Clone)]
pub struct RemoteSetting {
    pub config: Config,
    pub reg: MessageRegistry,
}

impl RemoteSetting {
    pub fn new(config: &Config, reg: MessageRegistry) -> Self {
        unimplemented!("TODO")
    }
}

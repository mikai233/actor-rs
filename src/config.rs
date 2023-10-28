#[derive(Debug, Copy, Clone)]
pub struct ActorConfig {
    pub mailbox: usize,
}

impl Default for ActorConfig {
    fn default() -> Self {
        ActorConfig {
            mailbox: 60000,
        }
    }
}
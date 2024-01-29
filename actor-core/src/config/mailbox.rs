use serde::{Deserialize, Serialize};

pub const SYSTEM_MAILBOX_SIZE: usize = 500;

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Mailbox {
    pub mailbox_capacity: usize,
    pub stash_capacity: Option<usize>,
    pub throughput: usize,
}

impl Default for Mailbox {
    fn default() -> Self {
        Self {
            mailbox_capacity: 5000,
            stash_capacity: Some(5000),
            throughput: 50,
        }
    }
}
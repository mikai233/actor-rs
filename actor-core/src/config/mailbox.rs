use serde::{Deserialize, Serialize};

use super::duration::Duration;

pub const SYSTEM_MAILBOX_SIZE: usize = 500;

#[derive(Debug, Copy, Clone, Serialize, Deserialize, derive_more::Constructor)]
pub struct Mailbox {
    pub mailbox_capacity: usize,
    pub mailbox_push_timeout_time: Duration,
    pub stash_capacity: Option<usize>,
    pub throughput: usize,
}

use serde::{Deserialize, Serialize};

use crate::util::duration::ConfigDuration;

pub const SYSTEM_MAILBOX_SIZE: usize = 500;

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Mailbox {
    pub mailbox_capacity: Option<usize>,
    pub mailbox_push_timeout_time: ConfigDuration,
    pub stash_capacity: Option<usize>,
    pub throughput: usize,
}

use std::fmt::Debug;

#[derive(Debug, Copy, Clone)]
pub struct Props {
    pub mailbox: usize,
    pub throughput: usize,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            mailbox: 6000,
            throughput: 10,
        }
    }
}
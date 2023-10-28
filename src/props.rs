use std::fmt::Debug;

#[derive(Debug, Copy, Clone)]
pub struct Props {
    pub mailbox: usize,
    pub throughput: usize,
}
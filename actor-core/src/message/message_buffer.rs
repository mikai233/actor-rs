use std::collections::VecDeque;
use std::ops::{Deref, DerefMut};

use ahash::HashMap;

#[derive(Debug, Default)]
pub struct MessageBufferMap<I, M> {
    pub buffer: HashMap<I, VecDeque<M>>,
}

impl<I, M> Deref for MessageBufferMap<I, M> {
    type Target = HashMap<I, VecDeque<M>>;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

impl<I, M> DerefMut for MessageBufferMap<I, M> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.buffer
    }
}
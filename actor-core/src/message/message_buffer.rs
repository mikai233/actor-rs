use std::collections::VecDeque;
use std::ops::{Deref, DerefMut};
use ahash::HashMap;
use crate::cell::envelope::Envelope;

type MessageBuffer = VecDeque<Envelope>;

#[derive(Debug, Default)]
pub struct MessageBufferMap<I> {
    pub buffer: HashMap<I, MessageBuffer>,
}

impl<I> Deref for MessageBufferMap<I> {
    type Target = HashMap<I, MessageBuffer>;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

impl<I> DerefMut for MessageBufferMap<I> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.buffer
    }
}
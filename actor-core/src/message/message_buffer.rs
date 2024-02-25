use std::collections::VecDeque;
use std::hash::Hash;
use std::ops::{Deref, DerefMut};

use ahash::HashMap;
use crate::actor::actor_ref::{ActorRef, ActorRefExt};
use crate::actor::dead_letter_listener::Dropped;
use crate::DynMessage;

pub trait BufferEnvelope {
    fn message(&self) -> &DynMessage;

    fn sender(&self) -> &Option<ActorRef>;

    fn into_inner(self) -> (DynMessage, Option<ActorRef>);
}

#[derive(Debug)]
pub struct MessageBufferMap<I, M> where I: Eq + Hash {
    pub buffers_by_key: HashMap<I, VecDeque<M>>,
}

impl<I, M> MessageBufferMap<I, M> where I: Eq + Hash, M: BufferEnvelope {
    pub fn drop(&mut self, id: &I, reason: String, dead_letters: ActorRef) -> usize {
        match self.buffers_by_key.remove(id) {
            None => {
                0
            }
            Some(buffers) => {
                let len = buffers.len();
                for msg in buffers {
                    let (msg, sender) = msg.into_inner();
                    let dropped = Dropped::new(msg, reason.clone(), sender);
                    dead_letters.cast_ns(dropped);
                }
                len
            }
        }
    }
}

impl<I, M> Deref for MessageBufferMap<I, M> where I: Eq + Hash {
    type Target = HashMap<I, VecDeque<M>>;

    fn deref(&self) -> &Self::Target {
        &self.buffers_by_key
    }
}

impl<I, M> DerefMut for MessageBufferMap<I, M> where I: Eq + Hash {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.buffers_by_key
    }
}

impl<I, M> Default for MessageBufferMap<I, M> where I: Eq + Hash {
    fn default() -> Self {
        Self {
            buffers_by_key: Default::default(),
        }
    }
}
use std::collections::hash_map::Entry;
use std::collections::VecDeque;
use std::hash::Hash;
use std::ops::{Deref, DerefMut};

use ahash::HashMap;

use crate::actor::actor_ref::{ActorRef, ActorRefExt};
use crate::actor::dead_letter_listener::Dropped;
use crate::{DynMessage, Message};

pub trait BufferEnvelope {
    type M;
    fn message(&self) -> &Self::M;

    fn sender(&self) -> &Option<ActorRef>;

    fn into_inner(self) -> (Self::M, Option<ActorRef>);
}

#[derive(Debug)]
pub struct MessageBufferMap<I, M> where I: Eq + Hash {
    pub buffers_by_key: HashMap<I, VecDeque<M>>,
}

impl<I, M> MessageBufferMap<I, M> where I: Eq + Hash, M: BufferEnvelope {
    pub fn drop(&mut self, id: &I, reason: String, dead_letters: ActorRef) -> usize where <M as BufferEnvelope>::M: Message {
        match self.buffers_by_key.remove(id) {
            None => {
                0
            }
            Some(buffers) => {
                let len = buffers.len();
                for msg in buffers {
                    let (msg, sender) = msg.into_inner();
                    let dropped = Dropped::new(DynMessage::user(msg), reason.clone(), sender);
                    dead_letters.cast_ns(dropped);
                }
                len
            }
        }
    }
}


impl<I, M> MessageBufferMap<I, M> where I: Eq + Hash {
    pub fn total_size(&self) -> usize {
        self.values().fold(0, |acc, buffer| { acc + buffer.len() })
    }

    pub fn push(&mut self, key: I, msg: M) {
        match self.buffers_by_key.entry(key) {
            Entry::Occupied(mut o) => {
                o.get_mut().push_back(msg);
            }
            Entry::Vacant(v) => {
                let mut queue = VecDeque::new();
                queue.push_back(msg);
                v.insert(queue);
            }
        }
    }

    pub fn remove_buffer(&mut self, key: &I) -> Option<VecDeque<M>> {
        self.buffers_by_key.remove(key)
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
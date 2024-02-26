use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::{Debug, Formatter};
use std::hash::Hash;
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

use actor_core::ext::type_name_of;

pub(crate) struct RecencyList<V> where V: Eq + Hash {
    recency: VecDeque<Node<V>>,
    lookup_node: HashMap<Rc<V>, Node<V>>,
}

impl<V> RecencyList<V> where V: Eq + Hash {
    pub(crate) fn len(&self) -> usize {
        self.lookup_node.len()
    }

    pub(crate) fn update(&mut self, value: Rc<V>) {
        match self.lookup_node.get_mut(&value) {
            None => {
                let node = Node {
                    value: value.clone(),
                    timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis(),
                };
                self.recency.push_back(node.clone());
                self.lookup_node.insert(value, node);
            }
            Some(node) => {
                node.timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
                self.recency.retain(|n| n != node);
            }
        }
    }
}

#[derive(PartialEq, Eq, Hash)]
struct Node<V> where V: Eq + Hash {
    value: Rc<V>,
    timestamp: u128,
}

impl<V> Debug for Node<V> where V: Debug + Eq + Hash {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let ty = type_name_of::<V>();
        f.debug_struct(&format!("Node<{ty}>"))
            .field("value", &self.value)
            .field("timestamp", &self.timestamp)
            .finish()
    }
}

impl<V> Clone for Node<V> where V: Eq + Hash {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            timestamp: self.timestamp,
        }
    }
}
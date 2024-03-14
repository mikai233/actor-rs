use std::collections::{HashMap, VecDeque};
use std::fmt::{Debug, Formatter};
use std::hash::Hash;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use itertools::Itertools;

use actor_core::ext::type_name_of;

pub struct RecencyList<V> where V: Eq + Hash + Clone {
    recency: VecDeque<Node<V>>,
    // front->less recent back->more recent
    lookup_node: HashMap<V, Node<V>>,
}

impl<V> RecencyList<V> where V: Eq + Hash + Clone {
    pub fn new() -> Self {
        Self {
            recency: Default::default(),
            lookup_node: Default::default(),
        }
    }

    pub fn len(&self) -> usize {
        self.lookup_node.len()
    }

    pub fn update(&mut self, value: V) {
        match self.lookup_node.get_mut(&value) {
            None => {
                let node = Node {
                    value: value.clone(),
                    timestamp: Self::current_millis(),
                };
                self.recency.push_back(node.clone());
                self.lookup_node.insert(value, node);
            }
            Some(node) => {
                self.recency.retain(|n| n != node);
                node.timestamp = Self::current_millis();
                self.recency.push_back(node.clone());
            }
        }
    }

    pub fn remove(&mut self, value: &V) {
        self.remove_node(value);
    }

    pub fn contains(&self, value: &V) -> bool {
        self.lookup_node.contains_key(value)
    }

    pub fn least_recent(&self) -> Option<&Node<V>> {
        self.recency.front()
    }

    pub fn most_recent(&self) -> Option<&Node<V>> {
        self.recency.back()
    }

    pub fn least_to_most_recent(&self) -> impl Iterator<Item=&Node<V>> {
        self.recency.iter()
    }

    pub fn most_to_least_recent(&self) -> impl Iterator<Item=&Node<V>> {
        self.recency.iter().rev()
    }

    pub fn remove_least_recent(&mut self, mut n: usize) -> Vec<Node<V>> {
        let mut nodes = vec![];
        while n > 0 {
            n -= 1;
            match self.recency.pop_front() {
                None => {
                    break;
                }
                Some(node) => {
                    nodes.push(node);
                }
            }
        }
        nodes
    }

    pub fn remove_most_recent(&mut self, mut n: usize) -> Vec<Node<V>> {
        let mut nodes = vec![];
        while n > 0 {
            n -= 1;
            match self.recency.pop_back() {
                None => {
                    break;
                }
                Some(node) => {
                    nodes.push(node)
                }
            }
        }
        nodes
    }

    pub fn remove_least_recent_outside(&mut self, duration: Duration) -> Vec<V> {
        let min = Self::current_millis() - duration.as_millis();
        let nodes = self.recency.iter()
            .filter(|n| n.timestamp < min)
            .map(|n| n.clone())
            .collect_vec();
        for node in &nodes {
            self.remove(&node.value);
        }
        nodes.into_iter().map(|n| n.value).collect()
    }

    pub fn remove_most_recent_within(&mut self, duration: Duration) -> Vec<V> {
        let max = Self::current_millis() - duration.as_millis();
        let nodes = self.recency.iter()
            .rev()
            .filter(|n| n.timestamp > max)
            .map(|n| n.clone())
            .collect_vec();
        for node in &nodes {
            self.remove(&node.value);
        }
        nodes.into_iter().map(|n| n.value).collect()
    }

    fn remove_node(&mut self, value: &V) -> Option<V> {
        match self.lookup_node.remove(value) {
            None => {
                None
            }
            Some(node) => {
                self.recency.retain(|n| {
                    &node != n
                });
                Some(node.value)
            }
        }
    }

    fn current_millis() -> u128 {
        SystemTime::now().duration_since(UNIX_EPOCH)
            .expect("system time before Unix epoch")
            .as_millis()
    }
}

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct Node<V> where V: Eq + Hash + Clone {
    value: V,
    timestamp: u128,
}

impl<V> Debug for Node<V> where V: Debug + Eq + Hash + Clone {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let ty = type_name_of::<V>();
        f.debug_struct(&format!("Node<{ty}>"))
            .field("value", &self.value)
            .field("timestamp", &self.timestamp)
            .finish_non_exhaustive()
    }
}

// impl<V> Clone for Node<V> where V: Eq + Hash + Clone {
//     fn clone(&self) -> Self {
//         Self {
//             value: self.value.clone(),
//             timestamp: self.timestamp,
//             prev: self.prev.clone(),
//             next: self.next.clone(),
//         }
//     }
// }

#[cfg(test)]
mod tests {
    use std::hash::Hash;

    use crate::entity_passivation_strategy::recency_list::RecencyList;

    #[test]
    fn test() {
        let mut list = RecencyList::new();
        list.update(1);
        list.update(2);
        list.update(3);
        let v = value(&list);
        assert_eq!(v, vec![1, 2, 3]);
        list.update(2);
        let v = value(&list);
        assert_eq!(v, vec![1, 3, 2]);
        list.update(4);
        let v = value(&list);
        assert_eq!(v, vec![1, 3, 2, 4]);
        list.update(4);
        let v = value(&list);
        assert_eq!(v, vec![1, 3, 2, 4]);
        assert_eq!(list.least_recent().unwrap().value, 1);
        assert_eq!(list.most_recent().unwrap().value, 4);
        let v = list.least_to_most_recent().map(|n| n.value).collect::<Vec<_>>();
        assert_eq!(v, vec![1, 3, 2, 4]);
        let v = list.most_to_least_recent().map(|n| n.value).collect::<Vec<_>>();
        assert_eq!(v, vec![4, 2, 3, 1]);
        list.remove_least_recent(2);
        let v = value(&list);
        assert_eq!(v, vec![2, 4]);
        list.update(1);
        list.update(3);
        let v = value(&list);
        assert_eq!(v, vec![2, 4, 1, 3]);
        list.remove_most_recent(2);
        let v = value(&list);
        assert_eq!(v, vec![2, 4]);
        list.remove_most_recent(10);
        let v = value(&list);
        assert!(v.is_empty());
    }

    fn value<V>(list: &RecencyList<V>) -> Vec<V> where V: Eq + Hash + Clone {
        list.recency.iter().map(|n| n.value.clone()).collect()
    }
}
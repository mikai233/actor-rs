use std::collections::{HashMap, VecDeque};
use std::fmt::{Debug, Formatter};
use std::hash::Hash;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use actor_core::ext::type_name_of;

/// TODO 标准库里的LinkedList没有提供O(1)复杂度的移除操作，暂时用VecDeque代替
/// 由于[V]需要同时保存到两个数据结构中，因此需要[V]是可以clone的，如果[V]的clone代价比较昂贵的话
/// 可以用[std::rc::Rc]包裹
pub struct RecencyList<V> where V: Eq + Hash + Clone {
    recency: VecDeque<Node<V>>,
    // front->less recent back->more recent
    lookup_node: HashMap<V, Node<V>>,
}

impl<V> RecencyList<V> where V: Eq + Hash + Clone {
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

    pub fn remove_least_recent_outside(&mut self, duration: Duration) -> Vec<Node<V>> {
        let min = Self::current_millis() - duration.as_millis();
        let nodes = self.recency.iter()
            .filter(|n| n.timestamp < min)
            .map(|n| n.clone())
            .collect::<Vec<_>>();
        for node in &nodes {
            self.remove(&node.value);
        }
        nodes
    }

    pub fn remove_most_recent_within(&mut self, duration: Duration) -> Vec<Node<V>> {
        let max = Self::current_millis() - duration.as_millis();
        let nodes = self.recency.iter()
            .rev()
            .filter(|n| n.timestamp > max)
            .map(|n| n.clone())
            .collect::<Vec<_>>();
        for node in &nodes {
            self.remove(&node.value);
        }
        nodes
    }

    fn remove_node(&mut self, value: &V) -> Option<Node<V>> {
        match self.lookup_node.remove(value) {
            None => {
                None
            }
            Some(node) => {
                self.recency.retain(|n| {
                    &node != n
                });
                Some(node)
            }
        }
    }

    fn current_millis() -> u128 {
        SystemTime::now().duration_since(UNIX_EPOCH)
            .expect("system time before Unix epoch")
            .as_millis()
    }
}

#[derive(PartialEq, Eq, Hash)]
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
            .finish()
    }
}

impl<V> Clone for Node<V> where V: Eq + Hash + Clone {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            timestamp: self.timestamp,
        }
    }
}
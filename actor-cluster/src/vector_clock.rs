use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use std::ops::Add;
use std::sync::OnceLock;

use itertools::Itertools;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Node(String);

impl Node {
    fn from_hash(hash: impl Into<String>) -> Self {
        Self(hash.into())
    }

    fn hash(name: impl Into<String>) -> String {
        let name = name.into();
        let mut hasher = Sha256::new();
        hasher.update(name);
        hasher.finalize().iter().map(|b| format!("{:02x}", b)).collect()
    }

    fn new(name: impl Into<String>) -> Self {
        Self(Self::hash(name))
    }
}

impl Display for Node {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Hash, Eq, PartialEq, Copy, Clone)]
pub(crate) enum Ordering {
    After,
    Before,
    Same,
    Concurrent,
    FullOrder,
}

impl Display for Ordering {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Ordering::After => write!(f, "After"),
            Ordering::Before => write!(f, "Before"),
            Ordering::Same => write!(f, "Same"),
            Ordering::Concurrent => write!(f, "Concurrent"),
            Ordering::FullOrder => write!(f, "FullOrder"),
        }
    }
}

static CMP_END_MARKER: OnceLock<(Node, i64)> = OnceLock::new();

#[derive(Debug, Default, Clone, Eq, PartialEq)]
struct VectorClock {
    versions: BTreeMap<Node, i64>,
}

impl VectorClock {
    fn new(versions: BTreeMap<Node, i64>) -> Self {
        Self {
            versions,
        }
    }

    fn cmp_end_maker() -> (&'static Node, &'static i64) {
        let (node, timestamp) = CMP_END_MARKER.get_or_init(|| {
            (Node::new("endmaker"), 0)
        });
        (node, timestamp)
    }

    fn concurrent(&self, that: &VectorClock) -> bool {
        self.compare_only_to(that, Ordering::Concurrent) == Ordering::Concurrent
    }

    fn before(&self, that: &VectorClock) -> bool {
        self.compare_only_to(that, Ordering::Before) == Ordering::Before
    }

    fn after(&self, that: &VectorClock) -> bool {
        self.compare_only_to(that, Ordering::After) == Ordering::After
    }

    fn same(&self, that: &VectorClock) -> bool {
        self.compare_only_to(that, Ordering::Same) == Ordering::Same
    }

    fn compare_only_to(&self, that: &VectorClock, order: Ordering) -> Ordering {
        fn next_or_else<T>(iter: &mut impl Iterator<Item=T>, default: impl FnOnce() -> T) -> T {
            iter.next().unwrap_or_else(default)
        }

        fn compare_next<'a, 'b>(nt1: (&'a Node, &'a i64), nt2: (&'b Node, &'b i64), current_order: Ordering, mut i1: impl Iterator<Item=(&'a Node, &'a i64)>, mut i2: impl Iterator<Item=(&'b Node, &'b i64)>, request_order: Ordering) -> Ordering {
            if request_order != Ordering::FullOrder && current_order != Ordering::Same && current_order != request_order {
                current_order
            } else if nt1 == VectorClock::cmp_end_maker() && nt2 == VectorClock::cmp_end_maker() {
                current_order
            } else if nt1 == VectorClock::cmp_end_maker() {
                if current_order == Ordering::After {
                    Ordering::Concurrent
                } else {
                    Ordering::Before
                }
            } else if nt2 == VectorClock::cmp_end_maker() {
                if current_order == Ordering::Before {
                    Ordering::Concurrent
                } else {
                    Ordering::After
                }
            } else {
                let nc = nt1.0.cmp(&nt2.0);
                match nc {
                    std::cmp::Ordering::Less => {
                        if current_order == Ordering::Before {
                            Ordering::Concurrent
                        } else {
                            compare_next(next_or_else(&mut i1, || VectorClock::cmp_end_maker()), nt2, Ordering::After, i1, i2, request_order)
                        }
                    }
                    std::cmp::Ordering::Equal => {
                        if nt1.1 == nt2.1 {
                            compare_next(next_or_else(&mut i1, || VectorClock::cmp_end_maker()), next_or_else(&mut i2, || VectorClock::cmp_end_maker()), current_order, i1, i2, request_order)
                        } else if nt1.1 < nt2.1 {
                            if current_order == Ordering::After {
                                Ordering::Concurrent
                            } else {
                                compare_next(next_or_else(&mut i1, || VectorClock::cmp_end_maker()), next_or_else(&mut i2, || VectorClock::cmp_end_maker()), Ordering::Before, i1, i2, request_order)
                            }
                        } else {
                            if current_order == Ordering::Before {
                                Ordering::Concurrent
                            } else {
                                compare_next(next_or_else(&mut i1, || VectorClock::cmp_end_maker()), next_or_else(&mut i2, || VectorClock::cmp_end_maker()), Ordering::After, i1, i2, request_order)
                            }
                        }
                    }
                    std::cmp::Ordering::Greater => {
                        if current_order == Ordering::After {
                            Ordering::Concurrent
                        } else {
                            compare_next(nt1, next_or_else(&mut i2, || VectorClock::cmp_end_maker()), Ordering::Before, i1, i2, request_order)
                        }
                    }
                }
            }
        }

        fn compare<'a, 'b>(mut i1: impl Iterator<Item=(&'a Node, &'a i64)>, mut i2: impl Iterator<Item=(&'b Node, &'b i64)>, request_order: Ordering) -> Ordering {
            compare_next(next_or_else(&mut i1, || VectorClock::cmp_end_maker()), next_or_else(&mut i2, || VectorClock::cmp_end_maker()), Ordering::Same, i1, i2, request_order)
        }

        if self == that || self.versions == that.versions {
            Ordering::Same
        } else {
            let order = if order == Ordering::Concurrent {
                Ordering::FullOrder
            } else {
                order
            };
            compare(self.versions.iter(), that.versions.iter(), order)
        }
    }

    fn compare_to(&self, that: &VectorClock) -> Ordering {
        self.compare_only_to(that, Ordering::FullOrder)
    }

    fn merge(&self, that: &VectorClock) -> VectorClock {
        let mut merged_versions = that.versions.clone();
        for (node, time) in &self.versions {
            let merged_versions_current_time = merged_versions.get(node).copied().unwrap_or(0);
            if *time > merged_versions_current_time {
                merged_versions.insert(node.clone(), *time);
            }
        }
        VectorClock::new(merged_versions)
    }

    fn prune(&self, removed_node: &Node) -> VectorClock {
        if self.versions.contains_key(removed_node) {
            let mut pruned_versions = self.versions.clone();
            pruned_versions.remove(removed_node);
            VectorClock::new(pruned_versions)
        } else {
            self.clone()
        }
    }
}

impl Add<Node> for VectorClock {
    type Output = Self;

    fn add(self, rhs: Node) -> Self::Output {
        let mut versions = self.versions.clone();
        let version = versions.entry(rhs).or_insert(0);
        *version += 1;
        VectorClock::new(versions)
    }
}

impl Display for VectorClock {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let versions = self.versions.iter().map(|(n, t)| format!("{} -> {}", n, t)).join(", ");
        write!(f, "VectorClock({})", versions)
    }
}

#[cfg(test)]
mod test {
    use std::collections::BTreeMap;

    use crate::vector_clock::{Node, VectorClock};

    #[test]
    fn have_zero_versions_when_created() {
        let clock = VectorClock::default();
        assert_eq!(clock.versions, BTreeMap::new());
    }

    #[test]
    fn not_happen_before_itself() {
        let clock1 = VectorClock::default();
        let clock2 = VectorClock::default();
        assert!(!clock1.concurrent(&clock2));
    }

    #[test]
    fn pass_misc_comparison_test1() {
        let clock1_1 = VectorClock::default();
        let clock2_1 = clock1_1 + Node::new("1");
        let clock3_1 = clock2_1 + Node::new("2");
        let clock4_1 = clock3_1 + Node::new("1");

        let clock1_2 = VectorClock::default();
        let clock2_2 = clock1_2 + Node::new("1");
        let clock3_2 = clock2_2 + Node::new("2");
        let clock4_2 = clock3_2 + Node::new("1");

        assert!(!clock4_1.concurrent(&clock4_2));
    }

    #[test]
    fn pass_misc_comparison_test2() {
        let clock1_1 = VectorClock::default();
        let clock2_1 = clock1_1 + Node::new("1");
        let clock3_1 = clock2_1 + Node::new("2");
        let clock4_1 = clock3_1 + Node::new("1");

        let clock1_2 = VectorClock::default();
        let clock2_2 = clock1_2 + Node::new("1");
        let clock3_2 = clock2_2 + Node::new("2");
        let clock4_2 = clock3_2 + Node::new("1");
        let clock5_2 = clock4_2 + Node::new("3");

        assert!(clock4_1.before(&clock5_2));
    }

    #[test]
    fn pass_misc_comparison_test3() {
        let clock1_1 = VectorClock::default();
        let clock2_1 = clock1_1 + Node::new("1");

        let clock1_2 = VectorClock::default();
        let clock2_2 = clock1_2 + Node::new("2");

        assert!(clock2_1.concurrent(&clock2_2));
    }

    #[test]
    fn pass_misc_comparison_test4() {
        let clock1_3 = VectorClock::default();
        let clock2_3 = clock1_3 + Node::new("1");
        let clock3_3 = clock2_3 + Node::new("2");
        let clock4_3 = clock3_3 + Node::new("1");

        let clock1_4 = VectorClock::default();
        let clock2_4 = clock1_4 + Node::new("1");
        let clock3_4 = clock2_4 + Node::new("1");
        let clock4_4 = clock3_4 + Node::new("3");

        assert!(clock4_3.concurrent(&clock4_4));
    }

    #[test]
    fn pass_misc_comparison_test5() {
        let clock1_1 = VectorClock::default();
        let clock2_1 = clock1_1 + Node::new("2");
        let clock3_1 = clock2_1 + Node::new("2");

        let clock1_2 = VectorClock::default();
        let clock2_2 = clock1_2 + Node::new("1");
        let clock3_2 = clock2_2 + Node::new("2");
        let clock4_2 = clock3_2 + Node::new("2");
        let clock5_2 = clock4_2 + Node::new("3");

        assert!(clock3_1.before(&clock5_2));
        assert!(clock5_2.after(&clock3_1));
    }

    #[test]
    fn pass_misc_comparison_test6() {
        let clock1_1 = VectorClock::default();
        let clock2_1 = clock1_1 + Node::new("1");
        let clock3_1 = clock2_1 + Node::new("2");

        let clock1_2 = VectorClock::default();
        let clock2_2 = clock1_2 + Node::new("1");
        let clock3_2 = clock2_2 + Node::new("1");

        assert!(clock3_1.concurrent(&clock3_2));
        assert!(clock3_2.concurrent(&clock3_1));
    }

    #[test]
    fn pass_misc_comparison_test7() {
        let clock1_1 = VectorClock::default();
        let clock2_1 = clock1_1 + Node::new("1");
        let clock3_1 = clock2_1 + Node::new("2");
        let clock4_1 = clock3_1 + Node::new("2");
        let clock5_1 = clock4_1.clone() + Node::new("3");

        let clock1_2 = clock4_1;
        let clock2_2 = clock1_2 + Node::new("2");
        let clock3_2 = clock2_2 + Node::new("2");

        assert!(clock5_1.concurrent(&clock3_2));
        assert!(clock3_2.concurrent(&clock5_1));
    }

    #[test]
    fn pass_misc_comparison_test8() {
        let clock1_1 = VectorClock::default();
        let clock2_1 = clock1_1 + Node::from_hash("1");
        let clock3_1 = clock2_1 + Node::from_hash("3");

        let clock1_2 = clock3_1.clone() + Node::from_hash("2");

        let clock4_1 = clock3_1 + Node::from_hash("3");

        assert!(clock4_1.concurrent(&clock1_2));
        assert!(clock1_2.concurrent(&clock4_1));
    }

    #[test]
    fn correctly_merge_two_clocks() {
        let node1 = Node::new("1");
        let node2 = Node::new("2");
        let node3 = Node::new("3");

        let clock1_1 = VectorClock::default();
        let clock2_1 = clock1_1 + node1.clone();
        let clock3_1 = clock2_1 + node2.clone();
        let clock4_1 = clock3_1 + node2.clone();
        let clock5_1 = clock4_1.clone() + node3.clone();

        let clock1_2 = clock4_1;
        let clock2_2 = clock1_2 + node2.clone();
        let clock3_2 = clock2_2 + node2.clone();

        let merged1 = clock3_2.merge(&clock5_1);
        assert_eq!(merged1.versions.len(), 3);
        assert!(merged1.versions.contains_key(&node1));
        assert!(merged1.versions.contains_key(&node2));
        assert!(merged1.versions.contains_key(&node3));

        let merged2 = clock5_1.merge(&clock3_2);
        assert_eq!(merged2.versions.len(), 3);
        assert!(merged2.versions.contains_key(&node1));
        assert!(merged2.versions.contains_key(&node2));
        assert!(merged2.versions.contains_key(&node3));

        assert!(clock3_2.before(&merged1));
        assert!(clock5_1.before(&merged1));

        assert!(clock3_2.before(&merged2));
        assert!(clock5_1.before(&merged2));

        assert!(merged1.same(&merged2));
    }
}
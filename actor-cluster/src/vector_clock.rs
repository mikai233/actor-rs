use std::collections::BTreeMap;
use std::ops::AddAssign;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Node(pub(crate) String);

impl Node {}

#[derive(Debug)]
pub(crate) enum Ordering {
    After,
    Before,
    Same,
    Concurrent,
    FullOrder,
}

#[derive(Debug)]
pub(crate) struct VectorClock {
    pub(crate) versions: BTreeMap<Node, i64>,
}

impl AddAssign<Node> for VectorClock {
    fn add_assign(&mut self, rhs: Node) {
        let version = self.versions.entry(rhs).or_insert(0);
        *version += 1;
    }
}
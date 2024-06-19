use ahash::{HashMap, HashMapExt, HashSet, HashSetExt};

use crate::unique_address::UniqueAddress;

#[derive(Debug, Clone)]
pub(crate) struct Reachability {
    pub(crate) records: Vec<Record>,
    pub(crate) versions: HashMap<UniqueAddress, i64>,
}

#[derive(Debug, Clone)]
pub(crate) struct Record {
    pub(crate) observer: UniqueAddress,
    pub(crate) subject: UniqueAddress,
    pub(crate) status: ReachabilityStatus,
    pub(crate) version: i64,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub(crate) enum ReachabilityStatus {
    Reachable,
    Unreachable,
    Terminated,
}

#[derive(Debug, Default)]
struct Cache {
    observer_rows_map: HashMap<UniqueAddress, HashMap<UniqueAddress, Record>>,
    all_unreachable: HashSet<UniqueAddress>,
    all_terminated: HashSet<UniqueAddress>,
    all_unreachable_or_terminated: HashSet<UniqueAddress>,
}

impl Cache {
    fn new(reachability: &Reachability) -> Self {
        let (observer_rows_map, all_unreachable, all_terminated) = if reachability.records.is_empty() {
            (HashMap::new(), HashSet::new(), HashSet::new())
        } else {
            let mut observer_rows_map = HashMap::new();
            let mut all_unreachable = HashSet::new();
            let mut all_terminated = HashSet::new();

            for record in &reachability.records {
                observer_rows_map
                    .entry(record.observer.clone())
                    .or_insert_with(HashMap::new)
                    .insert(record.subject.clone(), record.clone());
                match record.status {
                    ReachabilityStatus::Reachable => {}
                    ReachabilityStatus::Unreachable => {
                        all_unreachable.insert(record.subject.clone());
                    }
                    ReachabilityStatus::Terminated => {
                        all_terminated.insert(record.subject.clone());
                    }
                }
            }
            (observer_rows_map, all_unreachable, all_terminated)
        };
        let all_unreachable_or_terminated = if all_terminated.is_empty() {
            all_unreachable.clone()
        } else {
            all_unreachable.union(&all_terminated).cloned().collect()
        };
        Self {
            observer_rows_map,
            all_unreachable,
            all_terminated,
            all_unreachable_or_terminated,
        }
    }
}
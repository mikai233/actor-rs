use std::cmp::max;

use ahash::{HashMap, HashMapExt, HashSet, HashSetExt};

use crate::unique_address::UniqueAddress;

#[derive(Debug, Clone)]
pub(crate) struct Reachability {
    pub(crate) records: Vec<Record>,
    pub(crate) versions: HashMap<UniqueAddress, i64>,
    cache: Cache,
}

impl Reachability {
    fn new(records: Vec<Record>, versions: HashMap<UniqueAddress, i64>) -> Self {
        let mut reachability = Reachability {
            records,
            versions,
            cache: Default::default(),
        };
        let cache = Cache::new(&reachability);
        reachability.cache = cache;
        reachability
    }

    fn observer_rows(&self, observer: &UniqueAddress) -> Option<&HashMap<UniqueAddress, Record>> {
        self.cache.observer_rows_map.get(observer)
    }

    fn current_version(&self, observer: &UniqueAddress) -> i64 {
        self.versions.get(observer).copied().unwrap_or(0)
    }

    fn next_version(&self, observer: &UniqueAddress) -> i64 {
        self.current_version(observer) + 1
    }

    fn change(&self, observer: &UniqueAddress, subject: &UniqueAddress, status: ReachabilityStatus) -> Reachability {
        let v = self.next_version(observer);
        let mut new_versions = self.versions.clone();
        new_versions.insert(observer.clone(), v);
        let new_record = Record {
            observer: observer.clone(),
            subject: subject.clone(),
            status,
            version: v,
        };
        match self.observer_rows(observer) {
            None if matches!(status, ReachabilityStatus::Reachable) => {
                self.clone()
            }
            None => {
                let mut records = self.records.clone();
                records.push(new_record);
                Reachability::new(records, new_versions)
            }
            Some(old_observer_rows) => {
                match old_observer_rows.get(subject) {
                    None => {
                        if matches!(status, ReachabilityStatus::Reachable) && old_observer_rows.values().all(|r| matches!(r.status, ReachabilityStatus::Reachable)) {
                            let records = self.records.iter().filter(|r| r.observer != *observer).cloned().collect();
                            Reachability::new(records, new_versions)
                        } else {
                            let mut records = self.records.clone();
                            records.push(new_record);
                            Reachability::new(records, new_versions)
                        }
                    }
                    Some(old_record) => {
                        if matches!(old_record.status, ReachabilityStatus::Terminated) || old_record.status == status {
                            self.clone()
                        } else {
                            if matches!(status, ReachabilityStatus::Reachable) && old_observer_rows.values().all(|r| matches!(r.status,ReachabilityStatus::Reachable) || r.subject == *subject) {
                                let records = self.records.iter().filter(|r| r.observer != *observer).cloned().collect();
                                Reachability::new(records, new_versions)
                            } else {
                                let mut records = self.records.clone();
                                for record in &mut records {
                                    if record == old_record {
                                        *record = new_record;
                                        break;
                                    }
                                }
                                Reachability::new(records, new_versions)
                            }
                        }
                    }
                }
            }
        }
    }

    fn merge(&self, allowed: HashSet<UniqueAddress>, other: Reachability) -> Reachability {
        let mut records = Vec::with_capacity(max(self.records.len(), other.records.len()));
        let mut new_versions = self.versions.clone();
        for observer in &allowed {
            let observer_version1 = self.current_version(observer);
            let observer_version2 = other.current_version(observer);
            match (self.observer_rows(observer), other.observer_rows(observer)) {
                (None, None) => {},
                (Some(rows1), Some(rows2)) => {
                    let rows = if observer_version1 > observer_version2 { rows1 } else { rows2 };
                    rows.values().filter(|r| allowed.contains(&r.subject)).for_each(|r| {
                        records.push(r.clone());
                    });
                }
                (Some(rows1), None) => {
                    if observer_version1 > observer_version2 {
                        rows1.values().filter(|r| allowed.contains(&r.subject)).for_each(|r| {
                            records.push(r.clone());
                        });
                    }
                }
                (None, Some(rows2)) => {
                    if observer_version2 > observer_version1 {
                        rows2.values().filter(|r| allowed.contains(&r.subject)).for_each(|r| {
                            records.push(r.clone());
                        });
                    }
                }
            }

            if observer_version2 > observer_version1 {
                new_versions.insert(observer.clone(), observer_version2);
            }
        }

        new_versions.retain(|k, _| allowed.contains(k));
        Reachability::new(records, new_versions)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
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

#[derive(Debug, Clone, Default)]
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
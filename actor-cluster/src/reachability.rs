use std::cmp::max;
use std::fmt::{Display, Formatter};

use ahash::{HashMap, HashMapExt, HashSet, HashSetExt};
use itertools::Itertools;

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

    fn remove(&self, nodes: Vec<UniqueAddress>, other: Reachability) -> Reachability {
        let nodes_set = nodes.into_iter().collect::<HashSet<_>>();
        let new_records = self.records.iter().filter(|r| !nodes_set.contains(&r.observer) || !nodes_set.contains(&r.subject)).cloned().collect();
        let new_versions = self.versions.iter().filter(|(k, _)| !nodes_set.contains(k)).map(|(k, v)| (k.clone(), *v)).collect();
        Reachability::new(new_records, new_versions)
    }

    fn remove_observers(&self, nodes: HashSet<UniqueAddress>) -> Reachability {
        if nodes.is_empty() {
            return self.clone();
        } else {
            let new_records = self.records.iter().filter(|r| !nodes.contains(&r.observer)).cloned().collect();
            let new_versions = self.versions.iter().filter(|(k, _)| !nodes.contains(k)).map(|(k, v)| (k.clone(), *v)).collect();
            Reachability::new(new_records, new_versions)
        }
    }

    fn filter_records(&self, predicate: impl Fn(&Record) -> bool) -> Reachability {
        let new_records = self.records.iter().filter(|r| predicate(r)).cloned().collect();
        Reachability::new(new_records, self.versions.clone())
    }

    fn status(&self, observer: &UniqueAddress, subject: &UniqueAddress) -> ReachabilityStatus {
        self.observer_rows(observer).and_then(|rows| rows.get(subject)).map(|r| r.status).unwrap_or(ReachabilityStatus::Reachable)
    }

    fn status_by_node(&self, node: &UniqueAddress) -> ReachabilityStatus {
        if self.cache.all_terminated.contains(node) {
            ReachabilityStatus::Terminated
        } else if self.cache.all_unreachable.contains(node) {
            ReachabilityStatus::Unreachable
        } else {
            ReachabilityStatus::Reachable
        }
    }

    fn is_reachable(&self, observer: &UniqueAddress, subject: &UniqueAddress) -> bool {
        matches!(self.status(observer, subject), ReachabilityStatus::Reachable)
    }

    fn is_all_unreachable(&self) -> bool {
        self.records.is_empty()
    }

    fn all_unreachable(&self) -> &HashSet<UniqueAddress> {
        &self.cache.all_unreachable
    }

    fn all_unreachable_or_terminated(&self) -> &HashSet<UniqueAddress> {
        &self.cache.all_unreachable_or_terminated
    }

    fn all_unreachable_from(&self, observer: &UniqueAddress) -> Option<HashSet<&UniqueAddress>> {
        self.observer_rows(observer).map(|rows| {
            rows.values().filter(|r| matches!(r.status, ReachabilityStatus::Unreachable)).map(|r| &r.subject).collect()
        })
    }

    fn observers_grouped_by_unreachable(&self) -> HashMap<&UniqueAddress, HashSet<&UniqueAddress>> {
        let mut observers_grouped_by_unreachable = HashMap::new();
        for (subject, records_for_subject) in self.records.iter().group_by(|r| &r.subject).into_iter() {
            let observers = records_for_subject.filter(|r| matches!(r.status, ReachabilityStatus::Unreachable)).map(|r| &r.observer).collect();
            observers_grouped_by_unreachable.insert(subject, observers);
        }
        observers_grouped_by_unreachable
    }

    fn all_observers(&self) -> HashSet<&UniqueAddress> {
        self.records.iter().map(|r| &r.observer).collect()
    }

    fn records_from(&self, observer: &UniqueAddress) -> Vec<&Record> {
        self.observer_rows(observer).map(|rows| rows.values().collect()).unwrap_or_default()
    }
}

impl PartialEq for Reachability {
    fn eq(&self, other: &Self) -> bool {
        self.records.len() == other.records.len() &&
            self.versions == other.versions &&
            self.cache.observer_rows_map == other.cache.observer_rows_map
    }
}

impl Eq for Reachability {}

impl Display for Reachability {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for (observer, _) in self.versions.iter().sorted_by_key(|(k, _)| *k) {
            if let Some(rows_map) = self.observer_rows(observer) {
                for (subject, record) in rows_map.iter().sorted_by_key(|(k, _)| *k) {
                    let aggregated = self.status_by_node(subject);
                    writeln!(f, "{} -> {}: {} [{}] ({})", observer.address, subject.address, record.status, aggregated, record.version)?;
                }
            }
        }
        Ok(())
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

impl Display for ReachabilityStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ReachabilityStatus::Reachable => write!(f, "Reachable"),
            ReachabilityStatus::Unreachable => write!(f, "Unreachable"),
            ReachabilityStatus::Terminated => write!(f, "Terminated"),
        }
    }
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
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

    fn empty() -> Self {
        Self::new(vec![], Default::default())
    }

    fn observer_rows(&self, observer: &UniqueAddress) -> Option<&HashMap<UniqueAddress, Record>> {
        self.cache.observer_rows_map.get(observer)
    }

    fn unreachable(&self, observer: &UniqueAddress, subject: &UniqueAddress) -> Reachability {
        self.change(observer, subject, ReachabilityStatus::Unreachable)
    }

    fn reachable(&self, observer: &UniqueAddress, subject: &UniqueAddress) -> Reachability {
        self.change(observer, subject, ReachabilityStatus::Reachable)
    }

    fn terminated(&self, observer: &UniqueAddress, subject: &UniqueAddress) -> Reachability {
        self.change(observer, subject, ReachabilityStatus::Terminated)
    }

    fn current_version(&self, observer: &UniqueAddress) -> i64 {
        self.versions.get(observer).copied().unwrap_or(0)
    }

    fn next_version(&self, observer: &UniqueAddress) -> i64 {
        self.current_version(observer) + 1
    }

    fn change(
        &self,
        observer: &UniqueAddress,
        subject: &UniqueAddress,
        status: ReachabilityStatus,
    ) -> Reachability {
        let v = self.next_version(observer);
        let mut new_versions = self.versions.clone();
        new_versions.insert(observer.clone(), v);
        let new_record = Record::new(observer.clone(), subject.clone(), status, v);
        match self.observer_rows(observer) {
            None if matches!(status, ReachabilityStatus::Reachable) => self.clone(),
            None => {
                let mut records = self.records.clone();
                records.push(new_record);
                Reachability::new(records, new_versions)
            }
            Some(old_observer_rows) => match old_observer_rows.get(subject) {
                None => {
                    if matches!(status, ReachabilityStatus::Reachable)
                        && old_observer_rows
                            .values()
                            .all(|r| matches!(r.status, ReachabilityStatus::Reachable))
                    {
                        let records = self
                            .records
                            .iter()
                            .filter(|r| r.observer != *observer)
                            .cloned()
                            .collect();
                        Reachability::new(records, new_versions)
                    } else {
                        let mut records = self.records.clone();
                        records.push(new_record);
                        Reachability::new(records, new_versions)
                    }
                }
                Some(old_record) => {
                    if matches!(old_record.status, ReachabilityStatus::Terminated)
                        || old_record.status == status
                    {
                        self.clone()
                    } else {
                        if matches!(status, ReachabilityStatus::Reachable)
                            && old_observer_rows.values().all(|r| {
                                matches!(r.status, ReachabilityStatus::Reachable)
                                    || r.subject == *subject
                            })
                        {
                            let records = self
                                .records
                                .iter()
                                .filter(|r| r.observer != *observer)
                                .cloned()
                                .collect();
                            Reachability::new(records, new_versions)
                        } else {
                            let mut new_records = self.records.clone();
                            for record in &mut new_records {
                                if record == old_record {
                                    *record = new_record;
                                    break;
                                }
                            }
                            Reachability::new(new_records, new_versions)
                        }
                    }
                }
            },
        }
    }

    pub(crate) fn merge(
        &self,
        allowed: &HashSet<UniqueAddress>,
        other: &Reachability,
    ) -> Reachability {
        let mut records = Vec::with_capacity(max(self.records.len(), other.records.len()));
        let mut new_versions = self.versions.clone();
        for observer in allowed {
            let observer_version1 = self.current_version(observer);
            let observer_version2 = other.current_version(observer);
            match (self.observer_rows(observer), other.observer_rows(observer)) {
                (None, None) => {}
                (Some(rows1), Some(rows2)) => {
                    let rows = if observer_version1 > observer_version2 {
                        rows1
                    } else {
                        rows2
                    };
                    rows.values()
                        .filter(|r| allowed.contains(&r.subject))
                        .for_each(|r| {
                            records.push(r.clone());
                        });
                }
                (Some(rows1), None) => {
                    if observer_version1 > observer_version2 {
                        rows1
                            .values()
                            .filter(|r| allowed.contains(&r.subject))
                            .for_each(|r| {
                                records.push(r.clone());
                            });
                    }
                }
                (None, Some(rows2)) => {
                    if observer_version2 > observer_version1 {
                        rows2
                            .values()
                            .filter(|r| allowed.contains(&r.subject))
                            .for_each(|r| {
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

    pub(crate) fn remove<'a>(&self, nodes: impl IntoIterator<Item=&'a UniqueAddress>) -> Reachability {
        let nodes_set = nodes.into_iter().collect::<HashSet<_>>();
        let new_records = self
            .records
            .iter()
            .filter(|r| !(nodes_set.contains(&r.observer) || nodes_set.contains(&r.subject)))
            .cloned()
            .collect();
        let mut new_versions = self.versions.clone();
        new_versions.retain(|k, _| !nodes_set.contains(k));
        Reachability::new(new_records, new_versions)
    }

    pub(crate) fn remove_observers(&self, nodes: HashSet<&UniqueAddress>) -> Reachability {
        if nodes.is_empty() {
            return self.clone();
        } else {
            let new_records = self
                .records
                .iter()
                .filter(|r| !nodes.contains(&r.observer))
                .cloned()
                .collect();
            let mut new_versions = self.versions.clone();
            new_versions.retain(|k, _| !nodes.contains(k));
            Reachability::new(new_records, new_versions)
        }
    }

    pub(crate) fn filter_records(&self, predicate: impl Fn(&Record) -> bool) -> Reachability {
        let new_records = self
            .records
            .iter()
            .filter(|r| predicate(r))
            .cloned()
            .collect();
        Reachability::new(new_records, self.versions.clone())
    }

    fn status(&self, observer: &UniqueAddress, subject: &UniqueAddress) -> ReachabilityStatus {
        self.observer_rows(observer)
            .and_then(|rows| rows.get(subject))
            .map(|r| r.status)
            .unwrap_or(ReachabilityStatus::Reachable)
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

    fn is_reachable_by_node(&self, node: &UniqueAddress) -> bool {
        self.is_all_unreachable() || !self.all_unreachable_or_terminated().contains(node)
    }

    pub(crate) fn is_reachable(&self, observer: &UniqueAddress, subject: &UniqueAddress) -> bool {
        matches!(
            self.status(observer, subject),
            ReachabilityStatus::Reachable
        )
    }

    fn is_all_unreachable(&self) -> bool {
        self.records.is_empty()
    }

    fn all_unreachable(&self) -> &HashSet<UniqueAddress> {
        &self.cache.all_unreachable
    }

    pub(crate) fn all_unreachable_or_terminated(&self) -> &HashSet<UniqueAddress> {
        &self.cache.all_unreachable_or_terminated
    }

    fn all_unreachable_from(&self, observer: &UniqueAddress) -> Option<HashSet<&UniqueAddress>> {
        self.observer_rows(observer).map(|rows| {
            rows.values()
                .filter(|r| matches!(r.status, ReachabilityStatus::Unreachable))
                .map(|r| &r.subject)
                .collect()
        })
    }

    fn observers_grouped_by_unreachable(&self) -> HashMap<&UniqueAddress, HashSet<&UniqueAddress>> {
        let mut observers_grouped_by_unreachable = HashMap::new();
        for (subject, records_for_subject) in
            self.records.iter().group_by(|r| &r.subject).into_iter()
        {
            let observers = records_for_subject
                .filter(|r| matches!(r.status, ReachabilityStatus::Unreachable))
                .map(|r| &r.observer)
                .collect::<HashSet<_>>();
            if !observers.is_empty() {
                observers_grouped_by_unreachable.insert(subject, observers);
            }
        }
        observers_grouped_by_unreachable
    }

    pub(crate) fn all_observers(&self) -> HashSet<&UniqueAddress> {
        self.records.iter().map(|r| &r.observer).collect()
    }

    fn records_from(&self, observer: &UniqueAddress) -> Vec<&Record> {
        self.observer_rows(observer)
            .map(|rows| rows.values().collect())
            .unwrap_or_default()
    }
}

impl PartialEq for Reachability {
    fn eq(&self, other: &Self) -> bool {
        self.records.len() == other.records.len()
            && self.versions == other.versions
            && self.cache.observer_rows_map == other.cache.observer_rows_map
    }
}

impl Eq for Reachability {}

impl Display for Reachability {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for (observer, _) in self.versions.iter().sorted_by_key(|(k, _)| *k) {
            if let Some(rows_map) = self.observer_rows(observer) {
                for (subject, record) in rows_map.iter().sorted_by_key(|(k, _)| *k) {
                    let aggregated = self.status_by_node(subject);
                    writeln!(
                        f,
                        "{} -> {}: {} [{}] ({})",
                        observer.address,
                        subject.address,
                        record.status,
                        aggregated,
                        record.version
                    )?;
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Hash, Clone, Eq, PartialEq)]
pub(crate) struct Record {
    pub(crate) observer: UniqueAddress,
    pub(crate) subject: UniqueAddress,
    pub(crate) status: ReachabilityStatus,
    pub(crate) version: i64,
}

impl Record {
    fn new(
        observer: UniqueAddress,
        subject: UniqueAddress,
        status: ReachabilityStatus,
        version: i64,
    ) -> Self {
        Self {
            observer,
            subject,
            status,
            version,
        }
    }
}

#[derive(Debug, Hash, Copy, Clone, Eq, PartialEq)]
enum ReachabilityStatus {
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
        let (observer_rows_map, all_unreachable, all_terminated) =
            if reachability.records.is_empty() {
                (HashMap::new(), HashSet::new(), HashSet::new())
            } else {
                let mut observer_rows_map = HashMap::new();
                let mut all_terminated = HashSet::new();
                let mut all_unreachable = HashSet::new();

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
                let all_unreachable = all_unreachable
                    .difference(&all_terminated)
                    .cloned()
                    .collect::<HashSet<_>>();
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

#[cfg(test)]
mod test {
    use std::net::{Ipv4Addr, SocketAddrV4};

    use ahash::{HashMap, HashMapExt, HashSet, HashSetExt};

    use actor_core::{hashmap, hashset};
    use actor_core::actor::address::{Address, Protocol};

    use crate::reachability::{Reachability, ReachabilityStatus, Record};
    use crate::unique_address::UniqueAddress;

    fn node_a() -> UniqueAddress {
        UniqueAddress {
            address: Address {
                protocol: Protocol::Akka,
                system: "sys".into(),
                addr: Some(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 2552)),
            },
            uid: 1,
        }
    }

    fn node_b() -> UniqueAddress {
        UniqueAddress {
            address: Address {
                protocol: Protocol::Akka,
                system: "sys".into(),
                addr: Some(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 2552)),
            },
            uid: 2,
        }
    }

    fn node_c() -> UniqueAddress {
        UniqueAddress {
            address: Address {
                protocol: Protocol::Akka,
                system: "sys".into(),
                addr: Some(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 2552)),
            },
            uid: 3,
        }
    }

    fn node_d() -> UniqueAddress {
        UniqueAddress {
            address: Address {
                protocol: Protocol::Akka,
                system: "sys".into(),
                addr: Some(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 2552)),
            },
            uid: 4,
        }
    }

    fn node_e() -> UniqueAddress {
        UniqueAddress {
            address: Address {
                protocol: Protocol::Akka,
                system: "sys".into(),
                addr: Some(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 2552)),
            },
            uid: 5,
        }
    }

    #[test]
    fn be_reachable_when_empty() {
        let reachability = Reachability::empty();
        assert_eq!(reachability.is_reachable_by_node(&node_a()), true);
        assert_eq!(reachability.all_unreachable().is_empty(), true);
    }

    #[test]
    fn be_unreachable_when_one_observed_unreachable() {
        let node_a = node_a();
        let node_b = node_b();
        let reachability = Reachability::empty().unreachable(&node_b, &node_a);
        assert_eq!(reachability.is_reachable_by_node(&node_a), false);
        let mut all_unreachable = HashSet::new();
        all_unreachable.insert(node_a);
        assert_eq!(reachability.all_unreachable(), &all_unreachable);
    }

    #[test]
    fn not_be_reachable_when_terminated() {
        let node_a = node_a();
        let node_b = node_b();
        let reachability = Reachability::empty().terminated(&node_b, &node_a);
        assert_eq!(reachability.is_reachable_by_node(&node_a), false);
        let all_terminated = hashset!(node_a);
        assert_eq!(
            reachability.all_unreachable_or_terminated(),
            &all_terminated
        );
    }

    #[test]
    fn not_change_terminated_entry() {
        let node_a = node_a();
        let node_b = node_b();
        let reachability = Reachability::empty().terminated(&node_b, &node_a);
        let new_reachability = reachability.reachable(&node_b, &node_a);
        assert_eq!(reachability, new_reachability);
        let new_reachability = reachability.unreachable(&node_b, &node_a);
        assert_eq!(reachability, new_reachability);
    }

    #[test]
    fn not_change_when_same_status() {
        let node_a = node_a();
        let node_b = node_b();
        let reachability = Reachability::empty().unreachable(&node_b, &node_a);
        let new_reachability = reachability.unreachable(&node_b, &node_a);
        assert_eq!(reachability, new_reachability);
    }

    #[test]
    fn be_unreachable_when_some_observed_unreachable_and_others_reachable() {
        let node_a = node_a();
        let node_b = node_b();
        let node_c = node_c();
        let node_d = node_d();
        let reachability = Reachability::empty()
            .unreachable(&node_b, &node_a)
            .reachable(&node_c, &node_a)
            .reachable(&node_d, &node_a);
        assert_eq!(reachability.is_reachable_by_node(&node_a), false);
    }

    #[test]
    fn be_reachable_when_all_observed_reachable_again() {
        let node_a = node_a();
        let node_b = node_b();
        let node_c = node_c();
        let reachability = Reachability::empty()
            .unreachable(&node_b, &node_a)
            .unreachable(&node_c, &node_a)
            .reachable(&node_b, &node_a)
            .reachable(&node_c, &node_a)
            .unreachable(&node_b, &node_c)
            .unreachable(&node_c, &node_b);
        assert_eq!(reachability.is_reachable_by_node(&node_a), true);
    }

    #[test]
    fn exclude_observations_from_specific_downed_nodes() {
        let node_a = node_a();
        let node_b = node_b();
        let node_c = node_c();
        let node_d = node_d();
        let reachability = Reachability::empty()
            .unreachable(&node_c, &node_a)
            .reachable(&node_c, &node_a)
            .unreachable(&node_c, &node_b)
            .unreachable(&node_b, &node_a)
            .unreachable(&node_b, &node_c);
        assert_eq!(reachability.is_reachable_by_node(&node_a), false);
        assert_eq!(reachability.is_reachable_by_node(&node_b), false);
        assert_eq!(reachability.is_reachable_by_node(&node_c), false);
        let all_unreachable_or_terminated = hashset! {
            node_a.clone(),
            node_b.clone(),
            node_c.clone(),
        };
        assert_eq!(
            reachability.all_unreachable_or_terminated(),
            &all_unreachable_or_terminated
        );
        let all_unreachable_or_terminated = hashset!(&node_b);
        assert_eq!(
            reachability
                .remove_observers(all_unreachable_or_terminated.clone())
                .all_unreachable_or_terminated(),
            &all_unreachable_or_terminated.iter().cloned().collect()
        );
    }

    #[test]
    fn be_pruned_when_all_records_of_an_observer_are_reachable() {
        let node_a = node_a();
        let node_b = node_b();
        let node_c = node_c();
        let node_d = node_d();
        let node_e = node_e();
        let reachability = Reachability::empty()
            .unreachable(&node_b, &node_a)
            .unreachable(&node_b, &node_c)
            .unreachable(&node_d, &node_c)
            .reachable(&node_b, &node_a)
            .reachable(&node_b, &node_c);
        assert_eq!(reachability.is_reachable_by_node(&node_a), true);
        assert_eq!(reachability.is_reachable_by_node(&node_c), false);
        assert_eq!(
            reachability.records,
            vec![Record::new(
                node_d.clone(),
                node_c.clone(),
                ReachabilityStatus::Unreachable,
                1
            )]
        );
        let reachability2 = reachability
            .unreachable(&node_b, &node_d)
            .unreachable(&node_b, &node_e);
        let mut records = HashSet::new();
        records.insert(Record::new(
            node_d.clone(),
            node_c.clone(),
            ReachabilityStatus::Unreachable,
            1,
        ));
        records.insert(Record::new(
            node_b.clone(),
            node_d.clone(),
            ReachabilityStatus::Unreachable,
            5,
        ));
        records.insert(Record::new(
            node_b.clone(),
            node_e.clone(),
            ReachabilityStatus::Unreachable,
            6,
        ));
        assert_eq!(
            reachability2.records.into_iter().collect::<HashSet<_>>(),
            records
        );
    }

    #[test]
    fn have_correct_aggregated_status() {
        let records = vec![
            Record::new(node_a(), node_b(), ReachabilityStatus::Reachable, 2),
            Record::new(node_c(), node_b(), ReachabilityStatus::Unreachable, 2),
            Record::new(node_a(), node_d(), ReachabilityStatus::Unreachable, 3),
            Record::new(node_d(), node_b(), ReachabilityStatus::Terminated, 4),
        ];
        let versions = hashmap! {
            node_a() => 3,
            node_c() => 3,
            node_d() => 4,
        };
        let reachability = Reachability::new(records, versions);
        assert_eq!(
            reachability.status_by_node(&node_a()),
            ReachabilityStatus::Reachable
        );
        assert_eq!(
            reachability.status_by_node(&node_b()),
            ReachabilityStatus::Terminated
        );
        assert_eq!(
            reachability.status_by_node(&node_d()),
            ReachabilityStatus::Unreachable
        );
    }

    #[test]
    fn have_correct_status_for_a_mix_of_nodes() {
        let node_a = node_a();
        let node_b = node_b();
        let node_c = node_c();
        let node_d = node_d();
        let node_e = node_e();
        let reachability = Reachability::empty()
            .unreachable(&node_b, &node_a)
            .unreachable(&node_c, &node_a)
            .unreachable(&node_d, &node_a)
            .unreachable(&node_c, &node_b)
            .reachable(&node_c, &node_b)
            .unreachable(&node_d, &node_b)
            .unreachable(&node_d, &node_c)
            .reachable(&node_d, &node_c)
            .reachable(&node_e, &node_d)
            .unreachable(&node_a, &node_e)
            .terminated(&node_b, &node_e);
        assert_eq!(
            reachability.status(&node_b, &node_a),
            ReachabilityStatus::Unreachable
        );
        assert_eq!(
            reachability.status(&node_c, &node_a),
            ReachabilityStatus::Unreachable
        );
        assert_eq!(
            reachability.status(&node_d, &node_a),
            ReachabilityStatus::Unreachable
        );

        assert_eq!(
            reachability.status(&node_c, &node_b),
            ReachabilityStatus::Reachable
        );
        assert_eq!(
            reachability.status(&node_d, &node_b),
            ReachabilityStatus::Unreachable
        );

        assert_eq!(
            reachability.status(&node_a, &node_e),
            ReachabilityStatus::Unreachable
        );
        assert_eq!(
            reachability.status(&node_b, &node_e),
            ReachabilityStatus::Terminated
        );

        assert_eq!(reachability.is_reachable_by_node(&node_a), false);
        assert_eq!(reachability.is_reachable_by_node(&node_b), false);
        assert_eq!(reachability.is_reachable_by_node(&node_c), true);
        assert_eq!(reachability.is_reachable_by_node(&node_d), true);
        assert_eq!(reachability.is_reachable_by_node(&node_e), false);

        let all_unreachable = hashset! {
            node_a.clone(),
            node_b.clone(),
        };
        assert_eq!(reachability.all_unreachable(), &all_unreachable);
        let all_unreachable_from = hashset!(node_e.clone());
        assert_eq!(
            reachability
                .all_unreachable_from(&node_a)
                .unwrap()
                .into_iter()
                .map(|a| a.clone())
                .collect::<HashSet<_>>(),
            all_unreachable_from
        );
        let all_unreachable_from = hashset!(node_a.clone());
        assert_eq!(
            reachability
                .all_unreachable_from(&node_b)
                .unwrap()
                .into_iter()
                .map(|a| a.clone())
                .collect::<HashSet<_>>(),
            all_unreachable_from
        );
        let all_unreachable_from = hashset!(node_a.clone());
        assert_eq!(
            reachability
                .all_unreachable_from(&node_c)
                .unwrap()
                .into_iter()
                .map(|a| a.clone())
                .collect::<HashSet<_>>(),
            all_unreachable_from
        );
        let all_unreachable_from = hashset! {
            node_a.clone(),
            node_b.clone(),
        };
        assert_eq!(
            reachability
                .all_unreachable_from(&node_d)
                .unwrap()
                .into_iter()
                .map(|a| a.clone())
                .collect::<HashSet<_>>(),
            all_unreachable_from
        );

        let expected_observers_grouped_by_unreachable = hashmap! {
            node_a.clone() => hashset!{
                node_b.clone(),
                node_c.clone(),
                node_d.clone(),
            },
            node_b.clone() => hashset!{
                node_d.clone(),
            },
            node_e.clone() => hashset!{
                node_a.clone(),
            },

        };
        let observers_grouped_by_unreachable = reachability
            .observers_grouped_by_unreachable()
            .into_iter()
            .map(|(k, v)| {
                (
                    k.clone(),
                    v.into_iter().map(|a| a.clone()).collect::<HashSet<_>>(),
                )
            })
            .collect::<HashMap<_, _>>();
        assert_eq!(
            observers_grouped_by_unreachable,
            expected_observers_grouped_by_unreachable
        );
    }

    #[test]
    fn merge_by_picking_latest_version_of_each_record() {
        let node_a = node_a();
        let node_b = node_b();
        let node_c = node_c();
        let node_d = node_d();
        let node_e = node_e();
        let r1 = Reachability::empty()
            .unreachable(&node_b, &node_a)
            .unreachable(&node_c, &node_d);
        let r2 = r1
            .reachable(&node_b, &node_a)
            .unreachable(&node_d, &node_e)
            .unreachable(&node_c, &node_a);
        let set = hashset! {
            node_a.clone(),
            node_b.clone(),
            node_c.clone(),
            node_d.clone(),
            node_e.clone(),
        };
        let merged = r1.merge(&set, &r2);

        assert_eq!(
            merged.status(&node_b, &node_a),
            ReachabilityStatus::Reachable
        );
        assert_eq!(
            merged.status(&node_c, &node_a),
            ReachabilityStatus::Unreachable
        );
        assert_eq!(
            merged.status(&node_c, &node_d),
            ReachabilityStatus::Unreachable
        );
        assert_eq!(
            merged.status(&node_d, &node_e),
            ReachabilityStatus::Unreachable
        );
        assert_eq!(
            merged.status(&node_e, &node_a),
            ReachabilityStatus::Reachable
        );

        assert_eq!(merged.is_reachable_by_node(&node_a), false);
        assert_eq!(merged.is_reachable_by_node(&node_d), false);
        assert_eq!(merged.is_reachable_by_node(&node_e), false);

        let merged2 = r2.merge(&set, &r1);
        assert_eq!(
            merged2.records.into_iter().collect::<HashSet<_>>(),
            merged.records.into_iter().collect::<HashSet<_>>()
        );
    }

    #[test]
    fn merge_by_taking_allowed_set_into_account() {
        let node_a = node_a();
        let node_b = node_b();
        let node_c = node_c();
        let node_d = node_d();
        let node_e = node_e();
        let r1 = Reachability::empty()
            .unreachable(&node_b, &node_a)
            .unreachable(&node_c, &node_d);
        let r2 = r1
            .reachable(&node_b, &node_a)
            .unreachable(&node_d, &node_e)
            .unreachable(&node_c, &node_a);
        let allowed = hashset! {
            node_a.clone(),
            node_b.clone(),
            node_c.clone(),
            node_e.clone(),
        };
        let merged = r1.merge(&allowed, &r2);

        assert_eq!(
            merged.status(&node_b, &node_a),
            ReachabilityStatus::Reachable
        );
        assert_eq!(
            merged.status(&node_c, &node_a),
            ReachabilityStatus::Unreachable
        );
        assert_eq!(
            merged.status(&node_c, &node_d),
            ReachabilityStatus::Reachable
        );
        assert_eq!(
            merged.status(&node_d, &node_e),
            ReachabilityStatus::Reachable
        );
        assert_eq!(
            merged.status(&node_e, &node_a),
            ReachabilityStatus::Reachable
        );

        assert_eq!(merged.is_reachable_by_node(&node_a), false);
        assert_eq!(merged.is_reachable_by_node(&node_d), true);
        assert_eq!(merged.is_reachable_by_node(&node_e), true);

        let versions = hashset! {
            node_b.clone(),
            node_c.clone(),
        };
        assert_eq!(
            merged.versions.keys().cloned().collect::<HashSet<_>>(),
            versions
        );

        let merged2 = r2.merge(&allowed, &r1);
        assert_eq!(
            merged2.records.into_iter().collect::<HashSet<_>>(),
            merged.records.into_iter().collect::<HashSet<_>>()
        );
        assert_eq!(merged2.versions, merged.versions);
    }

    #[test]
    fn merge_correctly_after_pruning() {
        let node_a = node_a();
        let node_b = node_b();
        let node_c = node_c();
        let node_d = node_d();
        let node_e = node_e();
        let r1 = Reachability::empty()
            .unreachable(&node_b, &node_a)
            .unreachable(&node_c, &node_d);
        let r2 = r1.unreachable(&node_a, &node_e);
        let r3 = r1.reachable(&node_b, &node_a);
        let set = hashset! {
            node_a.clone(),
            node_b.clone(),
            node_c.clone(),
            node_d.clone(),
            node_e.clone(),
        };
        let merged = r2.merge(&set, &r3);
        let records = hashset! {
            Record::new(node_a.clone(), node_e.clone(), ReachabilityStatus::Unreachable, 1),
            Record::new(node_c.clone(), node_d.clone(), ReachabilityStatus::Unreachable, 1),
        };
        assert_eq!(merged.records.into_iter().collect::<HashSet<_>>(), records);

        let merged3 = r3.merge(&set, &r2);
        assert_eq!(merged3.records.into_iter().collect::<HashSet<_>>(), records);
    }

    #[test]
    fn merge_versions_correctly() {
        let node_a = node_a();
        let node_b = node_b();
        let node_c = node_c();
        let node_d = node_d();
        let node_e = node_e();
        let versions = hashmap! {
            node_a.clone() => 3,
            node_b.clone() => 5,
            node_c.clone() => 7,
        };
        let r1 = Reachability::new(vec![], versions);
        let versions = hashmap! {
            node_a.clone() => 6,
            node_b.clone() => 2,
            node_d.clone() => 1,
        };
        let r2 = Reachability::new(vec![], versions);
        let set = hashset! {
            node_a.clone(),
            node_b.clone(),
            node_c.clone(),
            node_d.clone(),
            node_e.clone(),
        };
        let merged = r1.merge(&set, &r2);

        let expected = hashmap! {
            node_a.clone() => 6,
            node_b.clone() => 5,
            node_c.clone() => 7,
            node_d.clone() => 1,
        };
        assert_eq!(merged.versions, expected);

        let merged2 = r2.merge(&set, &r1);
        assert_eq!(merged2.versions, expected);
    }

    #[test]
    fn remove_node() {
        let node_a = node_a();
        let node_b = node_b();
        let node_c = node_c();
        let node_d = node_d();
        let node_e = node_e();
        let r = Reachability::empty()
            .unreachable(&node_b, &node_a)
            .unreachable(&node_c, &node_d)
            .unreachable(&node_b, &node_c)
            .unreachable(&node_b, &node_e)
            .remove(vec![&node_a, &node_b]);

        assert_eq!(
            r.status(&node_b, &node_a),
            ReachabilityStatus::Reachable
        );
        assert_eq!(
            r.status(&node_c, &node_d),
            ReachabilityStatus::Unreachable
        );
        assert_eq!(
            r.status(&node_b, &node_c),
            ReachabilityStatus::Reachable
        );
        assert_eq!(
            r.status(&node_b, &node_e),
            ReachabilityStatus::Reachable
        );
    }

    #[test]
    fn remove_correctly_after_pruning() {
        let node_a = node_a();
        let node_b = node_b();
        let node_c = node_c();
        let node_d = node_d();
        let r = Reachability::empty()
            .unreachable(&node_b, &node_a)
            .unreachable(&node_b, &node_c)
            .unreachable(&node_d, &node_c)
            .reachable(&node_b, &node_a)
            .reachable(&node_b, &node_c);
        assert_eq!(
            r.records,
            vec![Record::new(
                node_d.clone(),
                node_c.clone(),
                ReachabilityStatus::Unreachable,
                1
            )]
        );
        let r2 = r.remove(vec![&node_b]);
        let set = hashset!(&node_d);
        assert_eq!(r2.all_observers(), set);
        assert_eq!(r2.versions.keys().collect::<HashSet::<_>>(), set);
    }

    #[test]
    fn be_able_to_filter_records() {
        let node_a = node_a();
        let node_b = node_b();
        let node_c = node_c();
        let r = Reachability::empty()
            .unreachable(&node_c, &node_b)
            .unreachable(&node_b, &node_a)
            .unreachable(&node_b, &node_c);
        let filtered1 = r.filter_records(|record| record.observer != node_c);
        assert_eq!(filtered1.is_reachable_by_node(&node_b), true);
        assert_eq!(filtered1.is_reachable_by_node(&node_a), false);
        let set = hashset!(&node_b);
        assert_eq!(filtered1.all_observers(), set);
    }
}

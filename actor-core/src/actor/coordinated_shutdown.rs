use std::collections::{BTreeSet, HashMap, HashSet};
use std::time::Duration;

use anyhow::anyhow;
use serde::{Deserialize, Serialize};

pub const PHASE_BEFORE_SERVICE_UNBIND: &'static str = "before-service-unbind";
pub const PHASE_SERVICE_UNBIND: &'static str = "service-unbind";
pub const PHASE_SERVICE_REQUESTS_DONE: &'static str = "service-requests-done";
pub const PHASE_SERVICE_STOP: &'static str = "service-stop";
pub const PHASE_BEFORE_CLUSTER_SHUTDOWN: &'static str = "before-cluster-shutdown";
pub const PHASE_CLUSTER_SHARDING_SHUTDOWN_REGION: &'static str = "cluster-sharding-shutdown-region";
pub const PHASE_CLUSTER_LEAVE: &'static str = "cluster-leave";
pub const PHASE_CLUSTER_EXITING: &'static str = "cluster-exiting";
pub const PHASE_CLUSTER_EXITING_DONE: &'static str = "cluster-exiting-done";
pub const PHASE_CLUSTER_SHUTDOWN: &'static str = "cluster-shutdown";
pub const PHASE_BEFORE_ACTOR_SYSTEM_TERMINATE: &'static str = "before-actor-system-terminate";
pub const PHASE_ACTOR_SYSTEM_TERMINATE: &'static str = "actor-system-terminate";

#[derive(Debug)]
pub struct CoordinatedShutdown {}

impl CoordinatedShutdown {
    fn topological_sort(phases: &HashMap<String, Phase>) -> anyhow::Result<Vec<String>> {
        let mut result = vec![];
        let mut unmarked = phases.keys().cloned().chain(phases.values().flat_map(|p| &p.depends_on).cloned()).collect::<BTreeSet<_>>();
        let mut temp_mark = HashSet::new();
        fn depth_first_search(result: &mut Vec<String>, phases: &HashMap<String, Phase>, unmarked: &mut BTreeSet<String>, temp_mark: &mut HashSet<String>, u: String) -> anyhow::Result<()> {
            if temp_mark.contains(&u) {
                return Err(anyhow!("Cycle detected in graph of phases. It must be a DAG. phase [{}] depends transitively on itself. All dependencies: {:?}", u, phases));
            }
            if unmarked.contains(&u) {
                temp_mark.insert(u.clone());
                if let Some(phase) = phases.get(&u) {
                    for u in &phase.depends_on {
                        depth_first_search(result, phases, unmarked, temp_mark, u.clone())?;
                    }
                }
                unmarked.remove(&u);
                temp_mark.remove(&u);
                result.push(u);
            }
            Ok(())
        }
        while let Some(head) = unmarked.pop_first() {
            depth_first_search(&mut result, phases, &mut unmarked, &mut temp_mark, head)?;
        }
        Ok(result)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Phase {
    depends_on: HashSet<String>,
    timeout: Duration,
    recover: bool,
    enabled: bool,
}

#[cfg(test)]
mod test {

}
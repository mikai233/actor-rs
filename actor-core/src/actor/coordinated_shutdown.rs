use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt::{Debug, Formatter};
use std::future::Future;
use std::time::Duration;

use anyhow::anyhow;
use serde::{Deserialize, Serialize};

use actor_derive::AsAny;

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

#[derive(Debug, AsAny)]
pub struct CoordinatedShutdown {
    phases: HashMap<String, Phase>,
    registered_phases: HashMap<String, PhaseTasks>,
}

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

    fn register<F>(&mut self, phase_name: String, name: String, fut: F) where F: Future<Output=()> + Send + 'static {
        let phase_tasks = self.registered_phases.entry(phase_name).or_insert(PhaseTasks::default());
        let task = TaskDefinition {
            name,
            task: Box::new(fut),
        };
        phase_tasks.tasks.push(task);
    }

    pub fn add_task<F>(&mut self, phase: String, task_name: String, fut: F) -> anyhow::Result<()> where F: Future<Output=()> + Send + 'static {
        if task_name.is_empty() {
            return Err(anyhow!("Set a task name when adding tasks to the Coordinated Shutdown. Try to use unique, self-explanatory names."));
        }

        todo!()
    }

    fn known_phases(&self) -> HashSet<&str> {
        todo!()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Phase {
    depends_on: HashSet<String>,
    timeout: Duration,
    recover: bool,
    enabled: bool,
}

#[derive(Default)]
struct PhaseTasks {
    tasks: Vec<TaskDefinition>,
}

impl Debug for PhaseTasks {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_struct("PhaseTasks")
            .field("tasks", &self.tasks)
            .finish()
    }
}

struct TaskDefinition {
    name: String,
    task: Box<dyn Future<Output=()> + Send>,
}

impl Debug for TaskDefinition {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_struct("TaskDefinition")
            .field("name", &self.name)
            .field("task", &"..")
            .finish()
    }
}

#[cfg(test)]
mod test {}
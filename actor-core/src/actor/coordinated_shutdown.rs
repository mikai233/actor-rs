use std::any::Any;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt::{Debug, Formatter};
use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use anyhow::anyhow;
use dashmap::mapref::one::{MappedRef, MappedRefMut};
use futures::future::BoxFuture;
use futures::FutureExt;
use serde::{Deserialize, Serialize};
use tracing::warn;

use actor_derive::AsAny;

use crate::actor::actor_system::ActorSystem;
use crate::actor::extension::Extension;
use crate::ext::as_any::AsAny;

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
    system: ActorSystem,
    registered_phases: HashMap<String, PhaseTasks>,
    ordered_phases: Vec<String>,
    run_started: AtomicBool,
}

impl CoordinatedShutdown {
    pub(crate) fn new(system: ActorSystem) -> Self {
        let ordered_phases = Self::topological_sort(&system.core_config().phases).unwrap();
        Self {
            system,
            registered_phases: HashMap::new(),
            ordered_phases,
            run_started: AtomicBool::new(false),
        }
    }

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

    fn register<F>(&mut self, phase_name: String, name: String, fut: F) where F: Future<Output=()> + Send + Sync + 'static {
        let phase_tasks = self.registered_phases.entry(phase_name).or_insert(PhaseTasks::default());
        let task = TaskDefinition {
            name,
            fut: fut.boxed(),
        };
        phase_tasks.tasks.lock().unwrap().push(task);
    }

    pub fn add_task<F>(&mut self, phase: impl Into<String>, task_name: impl Into<String>, fut: F) -> anyhow::Result<()> where F: Future<Output=()> + Send + Sync + 'static {
        let phase = phase.into();
        let known_phases = self.known_phases();
        if !known_phases.contains(&phase) {
            return Err(anyhow!("Unknown phase [{}], known phases [{:?}]. All phases (alone with their optional dependencies) mut be defined in configuration",phase, known_phases));
        }
        let task_name = task_name.into();
        if task_name.is_empty() {
            return Err(anyhow!("Set a task name when adding tasks to the Coordinated Shutdown. Try to use unique, self-explanatory names."));
        }
        self.register(phase, task_name, fut);
        Ok(())
    }

    fn known_phases(&self) -> HashSet<&String> {
        let mut phases = HashSet::new();
        for (name, phase) in &self.system.core_config().phases {
            phases.insert(name);
            for depends in &phase.depends_on {
                phases.insert(depends);
            }
        }
        phases
    }

    pub fn get(system: &ActorSystem) -> MappedRef<&'static str, Box<dyn Extension>, Self> {
        system.get_extension::<Self>().expect("CoordinatedShutdown extension not found")
    }

    pub fn get_mut(system: &ActorSystem) -> MappedRefMut<&'static str, Box<dyn Extension>, Self> {
        system.get_extension_mut::<Self>().expect("CoordinatedShutdown extension not found")
    }

    pub async fn run(&mut self, reason: Box<dyn Reason>) {
        let started = self.run_started.swap(true, Ordering::Relaxed);
        if !started {
            for phase_name in &self.ordered_phases {
                if let Some(phase) = self.system.core_config().phases.get(phase_name) {
                    if phase.enabled {
                        if let Some(phase_task) = self.registered_phases.remove(phase_name) {
                            let mut task = phase_task.tasks.lock().unwrap();
                            let tasks = task.drain(..);
                            for task in tasks {
                                if let Some(_) = tokio::time::timeout(phase.timeout, task.fut).await.err() {
                                    warn!("running phase {} task timout after {:?}", phase_name, phase.timeout);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Phase {
    pub depends_on: HashSet<String>,
    pub timeout: Duration,
    pub recover: bool,
    pub enabled: bool,
}

impl Default for Phase {
    fn default() -> Self {
        Self {
            depends_on: HashSet::new(),
            timeout: Duration::from_secs(10),
            recover: true,
            enabled: true,
        }
    }
}

#[derive(Default)]
struct PhaseTasks {
    tasks: Mutex<Vec<TaskDefinition>>,
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
    fut: BoxFuture<'static, ()>,
}

impl Debug for TaskDefinition {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_struct("TaskDefinition")
            .field("name", &self.name)
            .field("task", &"..")
            .finish()
    }
}

pub trait Reason: Debug + Any + AsAny {}

#[derive(Debug, AsAny)]
pub struct ActorSystemTerminateReason;

impl Reason for ActorSystemTerminateReason {}

#[derive(Debug, AsAny)]
pub struct CtrlCExitReason;

impl Reason for CtrlCExitReason {}

#[cfg(test)]
mod test {}
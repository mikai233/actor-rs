use std::any::type_name;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt::{Debug, Display, Formatter};
use std::future::Future;
use std::ops::Deref;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use eyre::{anyhow, Error};
use futures::future::{BoxFuture, join_all};
use futures::FutureExt;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

use actor_derive::AsAny;

use crate::actor::actor_system::{ActorSystem, WeakActorSystem};
use crate::actor::extension::Extension;
use crate::actor_ref::actor_ref_factory::ActorRefFactory;

pub const PHASE_BEFORE_SERVICE_UNBIND: &str = "before-service-unbind";
pub const PHASE_SERVICE_UNBIND: &str = "service-unbind";
pub const PHASE_SERVICE_REQUESTS_DONE: &str = "service-requests-done";
pub const PHASE_SERVICE_STOP: &str = "service-stop";
pub const PHASE_BEFORE_CLUSTER_SHUTDOWN: &str = "before-cluster-shutdown";
pub const PHASE_CLUSTER_SHARDING_SHUTDOWN_REGION: &str = "cluster-sharding-shutdown-region";
pub const PHASE_CLUSTER_LEAVE: &str = "cluster-leave";
pub const PHASE_CLUSTER_EXITING: &str = "cluster-exiting";
pub const PHASE_CLUSTER_EXITING_DONE: &str = "cluster-exiting-done";
pub const PHASE_CLUSTER_SHUTDOWN: &str = "cluster-shutdown";
pub const PHASE_BEFORE_ACTOR_SYSTEM_TERMINATE: &str = "before-actor-system-terminate";
pub const PHASE_ACTOR_SYSTEM_TERMINATE: &str = "actor-system-terminate";

#[derive(Debug, Clone, AsAny)]
pub struct CoordinatedShutdown {
    inner: Arc<Inner>,
}

#[derive(Debug)]
pub struct Inner {
    system: WeakActorSystem,
    registered_phases: Mutex<HashMap<String, PhaseTask>>,
    ordered_phases: Vec<String>,
    run_started: AtomicBool,
}

impl Deref for CoordinatedShutdown {
    type Target = Arc<Inner>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl CoordinatedShutdown {
    pub(crate) fn new(system: ActorSystem) -> eyre::Result<Self> {
        let ordered_phases = Self::topological_sort(&system.core_config().phases)?;
        let inner = Inner {
            system: system.downgrade(),
            registered_phases: Default::default(),
            ordered_phases,
            run_started: Default::default(),
        };
        let shutdown = Self {
            inner: inner.into(),
        };
        shutdown.init_ctrl_c_signal()?;
        shutdown.init_phase_actor_system_terminate(system)?;
        Ok(shutdown)
    }

    fn topological_sort(phases: &HashMap<String, Phase>) -> eyre::Result<Vec<String>> {
        let mut result = vec![];
        let mut unmarked = phases.keys()
            .cloned()
            .chain(phases.values().flat_map(|p| &p.depends_on).cloned())
            .collect::<BTreeSet<_>>();
        let mut temp_mark = HashSet::new();
        fn depth_first_search(
            result: &mut Vec<String>,
            phases: &HashMap<String, Phase>,
            unmarked: &mut BTreeSet<String>,
            temp_mark: &mut HashSet<String>,
            u: String,
        ) -> eyre::Result<()> {
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
                result.push(u.clone());
            }
            Ok(())
        }
        while let Some(head) = unmarked.first().cloned() {
            depth_first_search(&mut result, phases, &mut unmarked, &mut temp_mark, head)?;
        }
        Ok(result)
    }

    fn register<F>(&self, phase_name: String, name: String, fut: F) where F: Future<Output=()> + Send + 'static {
        let mut registered_phases = self.registered_phases.lock();
        let phase_tasks = registered_phases.entry(phase_name).or_insert(PhaseTask::default());
        let task = TaskDefinition {
            name,
            fut: fut.boxed(),
        };
        phase_tasks.tasks.push(task);
    }

    pub fn add_task<F>(
        &self,
        system: &ActorSystem,
        phase: impl Into<String>,
        task_name: impl Into<String>,
        fut: F,
    ) -> eyre::Result<()> where F: Future<Output=()> + Send + 'static {
        let phase = phase.into();
        let known_phases = Self::known_phases(system);
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

    fn known_phases(system: &ActorSystem) -> HashSet<String> {
        let mut know_phases = HashSet::new();
        let phases = system.core_config().phases.clone();
        for (name, phase) in phases {
            know_phases.insert(name);
            for depends in &phase.depends_on {
                know_phases.insert(depends.clone());
            }
        }
        know_phases
    }

    pub fn get(system: &ActorSystem) -> Self {
        system.get_ext::<Self>().expect(&format!("{} not found", type_name::<Self>()))
    }

    pub fn run<R: Reason + 'static>(&self, reason: R) -> impl Future<Output=()> {
        self.inner_run(Box::new(reason))
    }

    fn inner_run(&self, reason: Box<dyn Reason>) -> impl Future<Output=()> {
        let started = self.run_started.swap(true, Ordering::Relaxed);
        let system = self.system.upgrade().unwrap();
        let mut run_tasks = vec![];
        if !started {
            info!("running coordinated shutdown with reason [{}]",  reason);
            let mut registered_phases = {
                let error = reason.into_error();
                if error.is_some() {
                    *system.termination_error.lock() = error;
                }
                self.registered_phases.lock()
                    .drain()
                    .collect::<HashMap<_, _>>()
            };
            let core_config = system.core_config();
            for phase_name in &self.ordered_phases {
                if let Some(phase) = core_config.phases.get(phase_name) {
                    if phase.enabled {
                        if let Some(phase_task) = registered_phases.remove(phase_name) {
                            let mut tasks = vec![];
                            for task in phase_task.into_inner() {
                                let TaskDefinition { name, fut } = task;
                                let task = TaskRun {
                                    name,
                                    phase: phase_name.clone(),
                                    task: fut,
                                };
                                tasks.push(task);
                            }
                            run_tasks.push((phase_name.clone(), phase.timeout, tasks));
                        }
                    }
                }
            }
        }
        async move {
            if !started {
                for (phase, timeout, tasks) in run_tasks {
                    let mut task_futures = vec![];
                    for task in tasks {
                        let fut = system.handle().spawn(async move {
                            let TaskRun { name, phase, task } = task;
                            debug!("execute task [{}] at phase [{}]", name, phase);
                            task.await;
                            debug!("execute task [{}] at phase [{}] done", name, phase);
                        });
                        task_futures.push(fut);
                    }
                    if tokio::time::timeout(timeout, join_all(task_futures)).await.err().is_some() {
                        warn!("execute phase [{}] timeout after [{:?}]",  phase, timeout);
                    }
                }
                debug!("execute coordinated shutdown complete");
            }
        }
    }

    pub fn is_terminating(&self) -> bool {
        self.run_started.load(Ordering::Relaxed)
    }

    fn init_phase_actor_system_terminate(&self, system: ActorSystem) -> eyre::Result<()> {
        let sys = system.clone();
        self.add_task(&system, PHASE_ACTOR_SYSTEM_TERMINATE, "terminate-system", async move {
            let provider = sys.provider();
            let mut termination_rx = provider.termination_rx();
            sys.final_terminated();
            let _ = termination_rx.recv().await;
            sys.when_terminated().await;
        })?;
        Ok(())
    }

    fn init_ctrl_c_signal(&self) -> eyre::Result<()> {
        let system = self.system.upgrade()?;
        let coordinated = self.clone();
        system.handle().spawn(async move {
            if let Some(error) = tokio::signal::ctrl_c().await.err() {
                error!("ctrl c signal error {}", error);
            }
            coordinated.run(CtrlCExitReason).await;
        });
        Ok(())
    }

    pub fn run_started(&self) -> bool {
        self.run_started.load(Ordering::Relaxed)
    }

    pub fn timeout(system: &ActorSystem, phase: &str) -> Option<Duration> {
        system.core_config().phases.get(phase).map(|p| { p.timeout })
    }
}

impl Extension for CoordinatedShutdown {}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Phase {
    pub depends_on: HashSet<String>,
    pub timeout: Duration,
    pub enabled: bool,
}

impl Default for Phase {
    fn default() -> Self {
        Self {
            depends_on: HashSet::new(),
            timeout: Duration::from_secs(10),
            enabled: true,
        }
    }
}

#[derive(Default)]
struct PhaseTask {
    tasks: Vec<TaskDefinition>,
}

impl PhaseTask {
    fn into_inner(self) -> Vec<TaskDefinition> {
        self.tasks
    }
}

impl Debug for PhaseTask {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_struct("PhaseTask")
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
            .finish_non_exhaustive()
    }
}

struct TaskRun {
    name: String,
    phase: String,
    task: BoxFuture<'static, ()>,
}

pub trait Reason: Send + Display {
    fn into_error(self: Box<Self>) -> Option<Error> {
        None
    }
}

#[derive(Debug)]
pub struct ActorSystemTerminateReason;

impl Display for ActorSystemTerminateReason {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", type_name::<Self>())
    }
}

impl Reason for ActorSystemTerminateReason {}

#[derive(Debug)]
pub struct CtrlCExitReason;

impl Display for CtrlCExitReason {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", type_name::<Self>())
    }
}

impl Reason for CtrlCExitReason {}

#[derive(Debug)]
pub struct ActorSystemStartFailedReason(pub Error);

impl Display for ActorSystemStartFailedReason {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", type_name::<Self>())
    }
}

impl Reason for ActorSystemStartFailedReason {}

#[derive(Debug)]
pub struct ClusterDowningReason;

impl Display for ClusterDowningReason {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", type_name::<Self>())
    }
}

impl Reason for ClusterDowningReason {}

#[derive(Debug)]
pub struct ClusterLeavingReason;

impl Display for ClusterLeavingReason {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", type_name::<Self>())
    }
}

impl Reason for ClusterLeavingReason {}
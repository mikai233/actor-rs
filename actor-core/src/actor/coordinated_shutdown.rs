use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};
use std::fmt::{Debug, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::task::{Context, Poll};
use std::time::Duration;

use anyhow::anyhow;
use dashmap::mapref::one::{MappedRef, MappedRefMut};
use futures::{FutureExt, ready};
use futures::future::BoxFuture;
use pin_project::pin_project;
use serde::{Deserialize, Serialize};
use tokio::time::error::Elapsed;
use tracing::{error, info};

use actor_derive::AsAny;

use crate::actor::actor_system::ActorSystem;
use crate::actor::extension::Extension;

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
    registered_phases: Mutex<HashMap<String, PhaseTask>>,
    ordered_phases: Vec<String>,
    run_started: AtomicBool,
}

impl CoordinatedShutdown {
    pub(crate) fn new(system: ActorSystem) -> Self {
        let ordered_phases = Self::topological_sort(&system.core_config().phases).unwrap();
        let mut coordinated_shutdown = Self {
            system,
            registered_phases: Mutex::default(),
            ordered_phases,
            run_started: AtomicBool::new(false),
        };
        coordinated_shutdown.init_phase_actor_system_terminate();
        coordinated_shutdown.init_ctrl_c_signal();
        coordinated_shutdown
    }

    fn topological_sort(phases: &HashMap<String, Phase>) -> anyhow::Result<Vec<String>> {
        let mut result = vec![];
        let mut unmarked = phases.keys().cloned().chain(phases.values().flat_map(|p| &p.depends_on).cloned()).collect::<BTreeSet<_>>();
        let mut temp_mark = HashSet::new();
        fn depth_first_search(
            result: &mut Vec<String>,
            phases: &HashMap<String, Phase>,
            unmarked: &mut BTreeSet<String>,
            temp_mark: &mut HashSet<String>,
            u: String,
        ) -> anyhow::Result<()> {
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

    fn register<F>(&mut self, phase_name: String, name: String, fut: F) where F: Future<Output=()> + Send + Sync + 'static {
        let mut registered_phases = self.registered_phases.lock().unwrap();
        let phase_tasks = registered_phases.entry(phase_name).or_insert(PhaseTask::default());
        let task = TaskDefinition {
            name,
            fut: fut.boxed(),
        };
        phase_tasks.tasks.push(task);
    }

    pub fn add_task<F>(
        &mut self,
        phase: impl Into<String>,
        task_name: impl Into<String>,
        fut: F,
    ) -> anyhow::Result<()> where F: Future<Output=()> + Send + Sync + 'static {
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

    pub fn run(&mut self, reason: &dyn Reason) -> impl Future<Output=()> + 'static {
        let name = self.system.name();
        info!("{} run CoordinatedShutdown with reason {:?}", name, reason);
        let mut coordinated_tasks = VecDeque::new();
        let started = self.run_started.swap(true, Ordering::Relaxed);
        if !started {
            let mut registered_phases = self.registered_phases.lock().unwrap().drain().collect::<HashMap<_, _>>();
            for phase_name in &self.ordered_phases {
                if let Some(phase) = self.system.core_config().phases.get(phase_name) {
                    if phase.enabled {
                        if let Some(phase_task) = registered_phases.remove(phase_name) {
                            for task in phase_task.into_inner() {
                                let task_run = TaskRun {
                                    phase: phase_name.clone(),
                                    timeout: phase.timeout,
                                    task: tokio::time::timeout(phase.timeout, task),
                                };
                                coordinated_tasks.push_back(task_run);
                            }
                        }
                    }
                }
            }
        }
        async {
            for task in coordinated_tasks {
                task.await;
            }
        }
    }

    fn init_phase_actor_system_terminate(&mut self) {
        self.add_task(PHASE_ACTOR_SYSTEM_TERMINATE, "terminate-system", async {
            todo!()
        }).unwrap();
    }

    fn init_ctrl_c_signal(&self) {
        let fut = CoordinatedShutdown::get_mut(&self.system).run(&CtrlCExitReason);
        self.system.system_rt().spawn(async move {
            if let Some(error) = tokio::signal::ctrl_c().await.err() {
                error!("ctrl c signal error {}", error);
            }
            fut.await;
        });
    }
}

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

#[pin_project]
struct TaskDefinition {
    name: String,
    #[pin]
    fut: BoxFuture<'static, ()>,
}

impl Future for TaskDefinition {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        ready!(this.fut.poll(cx));
        Poll::Ready(())
    }
}

impl Debug for TaskDefinition {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_struct("TaskDefinition")
            .field("name", &self.name)
            .field("fut", &"..")
            .finish()
    }
}

#[derive(Debug)]
#[pin_project]
struct TaskRun {
    phase: String,
    timeout: Duration,
    #[pin]
    task: tokio::time::Timeout<TaskDefinition>,
}

impl Future for TaskRun {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let this = self.project();
        let r: Result<(), Elapsed> = ready!(this.task.poll(cx));
        if r.err().is_some() {
            error!("execute {} task timeout after {:?}", this.phase, this.timeout);
        }
        Poll::Ready(())
    }
}

pub trait Reason: Debug {}

#[derive(Debug, AsAny)]
pub struct ActorSystemTerminateReason;

impl Reason for ActorSystemTerminateReason {}

#[derive(Debug, AsAny)]
pub struct CtrlCExitReason;

impl Reason for CtrlCExitReason {}
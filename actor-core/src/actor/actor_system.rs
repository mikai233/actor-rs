use std::fmt::{Debug, Formatter};
use std::future::Future;
use std::ops::Deref;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use arc_swap::{ArcSwap, ArcSwapOption};
use dashmap::mapref::one::MappedRef;
use futures::FutureExt;
use rand::random;
use tokio::runtime::Runtime;
use tokio::sync::oneshot::{channel, Receiver, Sender};

use crate::actor::{system_guardian, user_guardian};
use crate::actor::actor_path::{ActorPath, TActorPath};
use crate::actor::actor_ref::{ActorRef, ActorRefExt};
use crate::actor::actor_ref_factory::ActorRefFactory;
use crate::actor::actor_ref_provider::ActorRefProvider;
use crate::actor::address::Address;
use crate::actor::config::actor_system_config::ActorSystemConfig;
use crate::actor::empty_actor_ref_provider::EmptyActorRefProvider;
use crate::actor::extension::{ActorSystemExtension, Extension};
use crate::actor::local_ref::LocalActorRef;
use crate::actor::props::Props;
use crate::actor::root_guardian::{AddShutdownHook, Shutdown};
use crate::actor::timer_scheduler::{TimerScheduler, TimerSchedulerActor};
use crate::event::event_bus::SystemEventBus;

#[derive(Clone)]
pub struct ActorSystem {
    inner: Arc<SystemInner>,
}

pub struct SystemInner {
    name: String,
    uid: i64,
    start_time: u128,
    provider: ArcSwap<ActorRefProvider>,
    runtime: Runtime,
    signal: RwLock<(Option<Sender<()>>, Option<Receiver<()>>)>,
    scheduler: ArcSwapOption<TimerScheduler>,
    event_bus: ArcSwapOption<SystemEventBus>,
    extensions: ActorSystemExtension,
}

impl Debug for ActorSystem {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_struct("ActorSystem")
            .field("name", &self.name)
            .field("start_time", &self.start_time)
            .field("provider", &self.provider)
            .field("runtime", &self.runtime)
            .field("scheduler", &self.scheduler)
            .field("event_bus", &self.event_bus)
            .field("extensions", &self.extensions)
            .finish()
    }
}

impl Deref for ActorSystem {
    type Target = Arc<SystemInner>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl ActorSystem {
    pub fn create(name: impl Into<String>, config: ActorSystemConfig) -> anyhow::Result<Self> {
        let ActorSystemConfig { provider_fn } = config;
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_name("actor-pool")
            .build()
            .expect("Failed building the Runtime");
        let (tx, rx) = channel();
        let inner = SystemInner {
            name: name.into(),
            uid: random(),
            start_time: SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis(),
            provider: ArcSwap::new(Arc::new(EmptyActorRefProvider.into())),
            runtime,
            signal: RwLock::new((Some(tx), Some(rx))),
            scheduler: ArcSwapOption::new(None),
            event_bus: ArcSwapOption::new(None),
            extensions: ActorSystemExtension::default(),
        };
        let system = Self {
            inner: inner.into(),
        };
        let (provider, spawns) = provider_fn(&system)?;
        system.provider.store(Arc::new(provider));
        for s in spawns {
            s.spawn(system.clone());
        }
        system.init_event_bus()?;
        system.init_scheduler()?;
        Ok(system)
    }
    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn address(&self) -> Address {
        self.provider().get_default_address().clone()
    }

    fn child(&self, child: &String) -> ActorPath {
        self.guardian().path.child(child)
    }

    fn descendant<'a>(&self, names: impl IntoIterator<Item=&'a str>) -> ActorPath {
        self.guardian().path.descendant(names)
    }

    pub fn start_time(&self) -> u128 {
        self.start_time
    }

    pub fn terminate(&self) {
        if let Some(tx) = self.signal.write().unwrap().0.take() {
            let _ = tx.send(());
        }
    }

    pub async fn wait_termination(&self) {
        let rx = self.signal.write().unwrap().1.take();
        let guardian = self.provider().root_guardian().clone();
        async fn stop(guardian: LocalActorRef) {
            let (tx, rx) = channel();
            guardian.cast(Shutdown { signal: tx }, ActorRef::no_sender());
            let _ = rx.await;
        }
        match rx {
            None => {
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => {
                        stop(guardian).await;
                    }
                }
            }
            Some(rx) => {
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => {
                        stop(guardian).await;
                    }
                    _ = rx => {
                        stop(guardian).await;
                    }
                }
            }
        }
    }

    pub fn system_guardian(&self) -> LocalActorRef {
        self.provider().system_guardian().clone()
    }

    pub fn runtime(&self) -> &Runtime {
        &self.runtime
    }

    pub fn spawn<F>(&self, future: F) -> tokio::task::JoinHandle<F::Output>
        where
            F: Future + Send + 'static,
            F::Output: Send + 'static, {
        self.runtime().spawn(future)
    }

    pub fn spawn_system_actor(&self, props: Props, name: Option<String>) -> anyhow::Result<ActorRef> {
        self.system_guardian().attach_child(props, name, true).map(|(actor, _)| actor)
    }

    pub fn event_bus(&self) -> Arc<SystemEventBus> {
        unsafe { self.event_bus.load().as_ref().unwrap_unchecked().clone() }
    }

    pub fn scheduler(&self) -> Arc<TimerScheduler> {
        unsafe { self.scheduler.load().as_ref().unwrap_unchecked().clone() }
    }

    pub fn add_shutdown_hook<F>(&self, fut: F) where F: Future<Output=()> + Send + 'static {
        self.provider().root_guardian().cast(AddShutdownHook { fut: fut.boxed() }, ActorRef::no_sender());
    }

    fn init_event_bus(&self) -> anyhow::Result<()> {
        let event_bus = SystemEventBus::new(self)?;
        self.event_bus.store(Some(event_bus.into()));
        Ok(())
    }

    fn init_scheduler(&self) -> anyhow::Result<()> {
        let timers = self.spawn_system_actor(Props::create(|context| TimerSchedulerActor::new(context.myself.clone())), Some("timers".to_string()))?;
        let scheduler = TimerScheduler::with_actor(timers);
        self.scheduler.store(Some(scheduler.into()));
        Ok(())
    }

    pub async fn register_extension<E, F, Fut>(&self, ext_fn: F) where E: Extension, F: Fn(ActorSystem) -> Fut, Fut: Future<Output=E> {
        let extension = ext_fn(self.clone()).await;
        self.extensions.register(extension);
    }

    pub fn get_extension<E>(&self) -> Option<MappedRef<&'static str, Box<dyn Extension>, E>> where E: Extension {
        self.extensions.get()
    }
}

impl ActorRefFactory for ActorSystem {
    fn system(&self) -> &ActorSystem {
        &self
    }

    fn provider(&self) -> Arc<ActorRefProvider> {
        self.provider.load_full()
    }

    fn guardian(&self) -> LocalActorRef {
        self.provider().guardian().clone()
    }

    fn lookup_root(&self) -> ActorRef {
        self.provider().root_guardian().clone().into()
    }

    fn spawn_actor(&self, props: Props, name: impl Into<String>) -> anyhow::Result<ActorRef> {
        self.guardian().attach_child(props, Some(name.into()), true).map(|(actor, _)| actor)
    }

    fn spawn_anonymous_actor(&self, props: Props) -> anyhow::Result<ActorRef> {
        self.guardian().attach_child(props, None, true).map(|(actor, _)| actor)
    }

    fn stop(&self, actor: &ActorRef) {
        let path = actor.path();
        let parent = path.parent();
        let guard = self.guardian();
        let sys = self.system_guardian();
        if parent == guard.path {
            guard.cast(
                user_guardian::StopChild {
                    child: actor.clone(),
                },
                None,
            );
        } else if parent == sys.path {
            sys.cast(
                system_guardian::StopChild {
                    child: actor.clone(),
                },
                None,
            );
        } else {
            actor.stop();
        }
    }
}

// #[cfg(test)]
// mod system_test {
//     use std::time::Duration;
//
//     use tracing::info;
//
//     use crate::{EmptyTestActor, EmptyTestMessage};
//     use crate::actor_path::TActorPath;
//     use crate::actor_ref::{ActorRef, ActorRefExt, TActorRef};
//     use crate::actor_ref_factory::ActorRefFactory;
//     use crate::props::Props;
//     use crate::system::ActorSystem;
//
//     #[tokio::test]
//     async fn test_spawn_actor() -> anyhow::Result<()> {
//         let system = ActorSystem::default();
//         for i in 0..10 {
//             let name = format!("testActor{}", i);
//             let actor = system.spawn_actor(Props::create(|_| EmptyTestActor), name)?;
//             let elements: Vec<String> = actor.path().elements();
//             info!("{:?}", elements);
//             tokio::spawn(async move {
//                 info!("{}", actor);
//                 loop {
//                     actor.cast(EmptyTestMessage, ActorRef::no_sender());
//                     tokio::time::sleep(Duration::from_secs(1)).await;
//                 }
//             });
//         }
//         tokio::time::sleep(Duration::from_secs(10)).await;
//         Ok(())
//     }
// }

use std::fmt::{Debug, Formatter};
use std::future::Future;
use std::ops::Deref;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use arc_swap::{ArcSwap, ArcSwapOption};
use etcd_client::Client;
use futures::FutureExt;
use tokio::runtime::Runtime;
use tokio::sync::oneshot::{channel, Receiver, Sender};

use crate::Actor;
use crate::actor_path::{ActorPath, TActorPath};
use crate::actor_ref::{ActorRef, ActorRefExt, TActorRef};
use crate::actor_ref::local_ref::LocalActorRef;
use crate::address::Address;
use crate::event::event_bus::SystemEventBus;
use crate::message::MessageRegistration;
use crate::props::{noarg_props, Props};
use crate::provider::{ActorRefFactory, ActorRefProvider, TActorRefProvider};
use crate::provider::empty_provider::EmptyActorRefProvider;
use crate::provider::remote_provider::RemoteActorRefProvider;
use crate::system::config::{Config, EtcdConfig};
use crate::system::root_guardian::{AddShutdownHook, Shutdown};
use crate::system::timer_scheduler::{TimerScheduler, TimerSchedulerActor};

pub mod root_guardian;
pub(crate) mod system_guardian;
pub(crate) mod user_guardian;
pub(crate) mod timer_scheduler;
pub mod config;

#[derive(Clone)]
pub struct ActorSystem {
    inner: Arc<Inner>,
}

pub struct Inner {
    address: Address,
    start_time: u128,
    pub(crate) provider: ArcSwap<ActorRefProvider>,
    registration: MessageRegistration,
    runtime: Runtime,
    client: Client,
    signal: RwLock<(Option<Sender<()>>, Option<Receiver<()>>)>,
    scheduler: ArcSwapOption<TimerScheduler>,
    event_bus: ArcSwapOption<SystemEventBus>,
}

impl Debug for ActorSystem {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ActorSystem")
            .field("address", &self.address)
            .field("start_time", &self.start_time)
            .field("provider", &self.provider)
            .field("reg", &self.registration)
            .field("runtime", &self.runtime)
            .field("client", &"..")
            .field("scheduler", &self.scheduler)
            .field("event_bus", &self.event_bus)
            .finish()
    }
}

impl Deref for ActorSystem {
    type Target = Arc<Inner>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl ActorSystem {
    pub async fn create(config: Config) -> anyhow::Result<Self> {
        let Config { name, addr, reg, etcd_config, } = config;
        let EtcdConfig { endpoints, connect_options } = etcd_config;
        let client = Client::connect(endpoints, connect_options).await?;
        let address = Address {
            protocol: "tcp".to_string(),
            system: name,
            addr,
        };
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_name("actor-pool")
            .build()
            .expect("Failed building the Runtime");
        let (tx, rx) = channel();
        let inner = Inner {
            start_time: SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis(),
            address,
            provider: ArcSwap::new(Arc::new(EmptyActorRefProvider::default().into())),
            registration: reg,
            runtime,
            client,
            signal: RwLock::new((Some(tx), Some(rx))),
            scheduler: ArcSwapOption::new(None),
            event_bus: ArcSwapOption::new(None),
        };
        let system = Self {
            inner: inner.into(),
        };
        RemoteActorRefProvider::init(&system)?;
        system.init_event_bus()?;
        system.init_scheduler()?;
        Ok(system)
    }
    pub fn name(&self) -> &String {
        &self.address.system
    }

    pub fn address(&self) -> &Address {
        &&self.address
    }

    fn child(&self, child: &String) -> ActorPath {
        self.guardian().path.child(child)
    }

    fn descendant(&self, names: Vec<String>) -> ActorPath {
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
            guardian.cast_async(Shutdown { signal: tx }, ActorRef::no_sender());
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

    pub(crate) fn system_guardian(&self) -> LocalActorRef {
        self.provider().system_guardian().clone()
    }

    pub(crate) fn registration(&self) -> &MessageRegistration {
        &self.registration
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

    pub(crate) fn system_actor_of<T>(&self, props: Props<T>, name: Option<String>) -> anyhow::Result<ActorRef> where T: Actor {
        self.system_guardian().attach_child(props, name)
    }

    pub fn event_bus(&self) -> Arc<SystemEventBus> {
        unsafe { self.event_bus.load().as_ref().unwrap_unchecked().clone() }
    }

    pub fn scheduler(&self) -> Arc<TimerScheduler> {
        unsafe { self.scheduler.load().as_ref().unwrap_unchecked().clone() }
    }

    pub fn eclient(&self) -> &Client {
        &self.client
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
        let timers = self.system_guardian().attach_child(noarg_props::<TimerSchedulerActor>(), Some("timers".to_string()))?;
        let scheduler = TimerScheduler::with_actor(timers);
        self.scheduler.store(Some(scheduler.into()));
        Ok(())
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

    fn actor_of<T>(&self, props: Props<T>, name: Option<String>) -> anyhow::Result<ActorRef>
        where
            T: Actor,
    {
        self.guardian().attach_child(props, name)
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

#[cfg(test)]
mod system_test {
    use std::time::Duration;

    use tracing::info;

    use crate::{EmptyTestActor, EmptyTestMessage};
    use crate::actor_path::TActorPath;
    use crate::actor_ref::{ActorRef, ActorRefExt, TActorRef};
    use crate::props::noarg_props;
    use crate::provider::ActorRefFactory;
    use crate::system::ActorSystem;
    use crate::system::config::Config;

    #[tokio::test]
    async fn test_spawn_actor() -> anyhow::Result<()> {
        let system = ActorSystem::create(Config::default()).await?;
        for i in 0..10 {
            let name = format!("testActor{}", i);
            let actor = system.actor_of(noarg_props::<EmptyTestActor>(), Some(name))?;
            let elements: Vec<String> = actor.path().elements();
            info!("{:?}", elements);
            tokio::spawn(async move {
                info!("{}", actor);
                loop {
                    actor.cast(EmptyTestMessage, ActorRef::no_sender());
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            });
        }
        tokio::time::sleep(Duration::from_secs(10)).await;
        Ok(())
    }
}

use std::any::type_name;
use std::fmt::{Debug, Formatter};
use std::future::Future;
use std::ops::Deref;
use std::pin::Pin;
use std::sync::{Arc, Weak};
use std::task::{Context, Poll};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context as _};
use arc_swap::{ArcSwap, Guard};
use dashmap::mapref::one::MappedRef;
use futures::future::BoxFuture;
use futures::FutureExt;
use parking_lot::Mutex;
use pin_project::pin_project;
use rand::random;
use tokio::sync::mpsc::{channel, Sender};

use crate::actor::address::Address;
use crate::actor::coordinated_shutdown::{ActorSystemTerminateReason, CoordinatedShutdown, Reason};
use crate::actor::extension::{Extension, SystemExtension};
use crate::actor::props::{ActorDeferredSpawn, Props};
use crate::actor::scheduler::{scheduler, SchedulerSender};
use crate::actor::system_guardian::SystemGuardian;
use crate::actor::user_guardian::UserGuardian;
use crate::actor_path::ActorPath;
use crate::actor_path::TActorPath;
use crate::actor_ref::{ActorRef, ActorRefExt, TActorRef};
use crate::actor_ref::actor_ref_factory::ActorRefFactory;
use crate::actor_ref::local_ref::LocalActorRef;
use crate::config::{ActorConfig, Config};
use crate::config::actor_setting::ActorSetting;
use crate::config::core_config::CoreConfig;
use crate::event::address_terminated_topic::AddressTerminatedTopic;
use crate::event::event_stream::EventStream;
use crate::ext::option_ext::OptionExt;
use crate::message::stop_child::StopChild;
use crate::provider::ActorRefProvider;
use crate::provider::empty_actor_ref_provider::EmptyActorRefProvider;

#[derive(Debug, Clone)]
pub struct ActorSystem {
    inner: Arc<Inner>,
}

#[derive(Debug)]
pub struct Inner {
    pub name: String,
    pub uid: i64,
    pub start_time: u128,
    provider: ArcSwap<ActorRefProvider>,
    pub scheduler: SchedulerSender,
    pub event_stream: EventStream,
    pub extension: SystemExtension,
    pub config: ActorConfig,
    signal: Sender<anyhow::Result<()>>,
    termination_callbacks: TerminationCallbacks,
    pub(crate) termination_error: Mutex<Option<anyhow::Error>>,
}

impl Deref for ActorSystem {
    type Target = Arc<Inner>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl ActorSystem {
    pub fn new(name: impl Into<String>, setting: ActorSetting) -> anyhow::Result<ActorSystemRunner> {
        let ActorSetting { provider, config: core_config } = setting;
        let scheduler = scheduler();
        let (signal_tx, mut signal_rx) = channel(1);
        let inner = Inner {
            name: name.into(),
            uid: random(),
            start_time: SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis(),
            provider: ArcSwap::new(Arc::new(EmptyActorRefProvider.into())),
            scheduler,
            event_stream: EventStream::default(),
            extension: SystemExtension::default(),
            config: ActorConfig::default(),
            signal: signal_tx,
            termination_callbacks: TerminationCallbacks::default(),
            termination_error: Mutex::default(),
        };
        let system = Self { inner: inner.into() };
        system.config.add(core_config)?;
        system.register_extension(|_| Ok(AddressTerminatedTopic::new()))?;
        let (provider, spawns) = provider(system.clone()).context("failed to create actor provider")?;
        system.provider.store(Arc::new(provider));
        system.register_extension(CoordinatedShutdown::new)?;
        for s in spawns {
            s.spawn(system.clone())?;
        }
        let signal = async move {
            match signal_rx.recv().await {
                None => Err(anyhow!("signal tx closed")),
                Some(result) => result,
            }
        }.boxed();
        let runner = ActorSystemRunner {
            system,
            signal,
        };
        Ok(runner)
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

    pub async fn terminate(&self) {
        self.run_coordinated_shutdown(ActorSystemTerminateReason).await;
    }

    pub(crate) fn when_terminated(&self) -> impl Future<Output=()> + 'static {
        let result = self.termination_error.lock()
            .take()
            .map(|e| { Err(e) })
            .unwrap_or(Ok(()));
        let callbacks = self.termination_callbacks.run();
        let signal = self.signal.clone();
        async move {
            callbacks.await;
            let _ = signal.send(result).await;
        }
    }

    pub(crate) fn final_terminated(&self) {
        self.provider().root_guardian().stop();
    }

    pub fn register_on_termination<F>(&self, fut: F) where F: Future<Output=()> + Send + 'static {
        self.termination_callbacks.add(fut);
    }

    pub fn system_guardian(&self) -> LocalActorRef {
        self.provider().system_guardian().clone()
    }

    pub fn spawn_system(&self, props: Props, name: Option<String>) -> anyhow::Result<ActorRef> {
        self.system_guardian().attach_child(props, name, None)
    }

    pub fn spawn_system_deferred(
        &self,
        props: Props,
        name: Option<String>,
    ) -> anyhow::Result<(ActorRef, ActorDeferredSpawn)> {
        self.system_guardian().attach_child_deferred_start(props, name, None)
    }

    pub fn register_extension<E, F>(&self, ext_fn: F) -> anyhow::Result<()> where E: Extension, F: Fn(ActorSystem) -> anyhow::Result<E> {
        let extension = ext_fn(self.clone())?;
        self.extension.register(extension)?;
        Ok(())
    }

    pub fn exist_ext<E>(&self) -> bool {
        self.extension.contains_key(type_name::<E>())
    }

    pub fn get_ext<E>(&self) -> Option<E> where E: Extension + Clone {
        self.extension.get()
    }

    pub fn get_config<C>(&self) -> MappedRef<&'static str, Box<dyn Config>, C> where C: Config {
        let msg = format!("{} not found", type_name::<CoreConfig>());
        self.config.get().expect(&msg)
    }

    pub fn add_config<C>(&self, config: C) -> anyhow::Result<()> where C: Config {
        self.config.add(config)
    }

    pub fn core_config(&self) -> CoreConfig {
        self.get_config::<CoreConfig>().clone()
    }

    pub fn dead_letters(&self) -> ActorRef {
        self.provider().dead_letters().clone()
    }

    pub fn downgrade(&self) -> WeakActorSystem {
        let inner = Arc::downgrade(&self.inner);
        WeakActorSystem { inner }
    }

    pub fn run_coordinated_shutdown<R>(&self, reason: R) -> impl Future<Output=()> where R: Reason + 'static {
        CoordinatedShutdown::get(self.system()).run(reason)
    }
}

impl ActorRefFactory for ActorSystem {
    fn system(&self) -> &ActorSystem {
        &self
    }

    fn provider_full(&self) -> Arc<ActorRefProvider> {
        self.provider.load_full()
    }

    fn provider(&self) -> Guard<Arc<ActorRefProvider>> {
        self.provider.load()
    }

    fn guardian(&self) -> LocalActorRef {
        self.provider().guardian().clone()
    }

    fn lookup_root(&self) -> ActorRef {
        self.provider().root_guardian().clone().into()
    }

    fn spawn(&self, props: Props, name: impl Into<String>) -> anyhow::Result<ActorRef> {
        self.guardian().attach_child(props, Some(name.into()), None)
    }

    fn spawn_anonymous(&self, props: Props) -> anyhow::Result<ActorRef> {
        self.guardian().attach_child(props, None, None)
    }

    fn stop(&self, actor: &ActorRef) {
        let path = actor.path();
        let parent = path.parent();
        let guard = self.guardian();
        let sys = self.system_guardian();
        if parent == guard.path {
            guard.cast_ns(StopChild::<UserGuardian>::new(actor.clone()));
        } else if parent == sys.path {
            sys.cast_ns(StopChild::<SystemGuardian>::new(actor.clone()));
        } else {
            actor.stop();
        }
    }
}

#[derive(Debug, Clone)]
pub struct WeakActorSystem {
    inner: Weak<Inner>,
}

impl WeakActorSystem {
    pub fn upgrade(&self) -> anyhow::Result<ActorSystem> {
        let inner = self.inner.upgrade()
            .into_result()
            .context("ActorSystem destroyed, any system week reference is invalid")?;
        Ok(ActorSystem { inner })
    }
}

#[pin_project]
pub struct ActorSystemRunner {
    pub system: ActorSystem,
    #[pin]
    pub signal: BoxFuture<'static, anyhow::Result<()>>,
}

impl Deref for ActorSystemRunner {
    type Target = ActorSystem;

    fn deref(&self) -> &Self::Target {
        &self.system
    }
}

impl Future for ActorSystemRunner {
    type Output = anyhow::Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let this = self.project();
        this.signal.poll(cx)
    }
}

impl Debug for ActorSystemRunner {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_struct("ActorSystemRunner")
            .field("system", &self.system)
            .finish_non_exhaustive()
    }
}

#[derive(Default)]
struct TerminationCallbacks {
    callbacks: Mutex<Vec<BoxFuture<'static, ()>>>,
}

impl TerminationCallbacks {
    fn add<F>(&self, fut: F) where F: Future<Output=()> + Send + 'static {
        self.callbacks.lock().push(fut.boxed());
    }

    fn run(&self) -> impl Future<Output=()> + 'static {
        let callbacks = self.callbacks.lock().drain(..).collect::<Vec<_>>();
        async move {
            for cb in callbacks {
                cb.await;
            }
        }
    }
}

impl Debug for TerminationCallbacks {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_struct("TerminationCallbacks")
            .field("callbacks", &"..")
            .finish()
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

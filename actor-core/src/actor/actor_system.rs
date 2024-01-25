use std::fmt::{Debug, Formatter};
use std::future::Future;
use std::ops::Deref;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::{SystemTime, UNIX_EPOCH};

use arc_swap::{ArcSwap, Guard};
use dashmap::mapref::one::{MappedRef, MappedRefMut};
use futures::{FutureExt, ready};
use futures::future::BoxFuture;
use pin_project::pin_project;
use rand::random;
use tokio::runtime::Runtime;

use crate::actor::actor_path::{ActorPath, TActorPath};
use crate::actor::actor_ref::{ActorRef, ActorRefExt, TActorRef};
use crate::actor::actor_ref_factory::ActorRefFactory;
use crate::actor::actor_ref_provider::ActorRefProvider;
use crate::actor::address::Address;
use crate::actor::config::actor_setting::ActorSetting;
use crate::actor::config::Config;
use crate::actor::config::core_config::CoreConfig;
use crate::actor::coordinated_shutdown::{ActorSystemTerminateReason, CoordinatedShutdown};
use crate::actor::empty_actor_ref_provider::EmptyActorRefProvider;
use crate::actor::extension::{ActorExtension, Extension};
use crate::actor::local_ref::LocalActorRef;
use crate::actor::props::Props;
use crate::actor::scheduler::{scheduler, SchedulerSender};
use crate::actor::system_guardian::SystemGuardian;
use crate::actor::user_guardian::UserGuardian;
use crate::CORE_CONFIG;
use crate::event::event_stream::EventStream;
use crate::message::stop_child::StopChild;

#[derive(Clone)]
pub struct ActorSystem {
    inner: Arc<SystemInner>,
}

pub struct SystemInner {
    name: String,
    uid: i64,
    start_time: u128,
    provider: ArcSwap<ActorRefProvider>,
    system_rt: Runtime,
    user_rt: Runtime,
    scheduler: SchedulerSender,
    event_stream: EventStream,
    extensions: ActorExtension,
    core_config: CoreConfig,
}

impl Debug for ActorSystem {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_struct("ActorSystem")
            .field("name", &self.name)
            .field("uid", &self.uid)
            .field("start_time", &self.start_time)
            .field("provider", &self.provider)
            .field("system_rt", &self.system_rt)
            .field("user_rt", &self.user_rt)
            .field("scheduler", &self.scheduler)
            .field("event_stream", &self.event_stream)
            .field("extensions", &self.extensions)
            .field("core_config", &self.core_config)
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
    pub fn create(name: impl Into<String>, setting: ActorSetting) -> anyhow::Result<ActorSystemRunner> {
        let ActorSetting { provider_fn, core_config } = setting;
        let default_config: CoreConfig = toml::from_str(CORE_CONFIG)?;
        let core_config = default_config.merge(core_config);
        let system_rt = Self::build_runtime("actor-system");
        let _guard = system_rt.enter();
        let scheduler = scheduler();
        let user_rt = Self::build_runtime("user-system");
        let inner = SystemInner {
            name: name.into(),
            uid: random(),
            start_time: SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis(),
            provider: ArcSwap::new(Arc::new(EmptyActorRefProvider.into())),
            system_rt,
            user_rt,
            scheduler,
            event_stream: EventStream::default(),
            extensions: ActorExtension::default(),
            core_config,
        };
        let system = Self {
            inner: inner.into(),
        };
        let (provider, spawns) = provider_fn(&system)?;
        let mut termination_rx = provider.termination_rx();
        system.provider.store(Arc::new(provider));
        system.register_extension(|system| {
            CoordinatedShutdown::new(system)
        });
        for s in spawns {
            s.spawn(system.clone());
        }
        let signal = async move {
            let _ = termination_rx.recv().await;
        }.boxed();
        let runner = ActorSystemRunner {
            system,
            signal,
        };
        Ok(runner)
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

    pub fn terminate(&self) -> impl Future<Output=()> {
        CoordinatedShutdown::get_mut(self.system()).run(&ActorSystemTerminateReason)
    }

    pub(crate) fn final_terminated(&self) {
        self.provider().root_guardian().stop();
    }

    // pub async fn wait_termination(&self) {
    //     let rx = self.signal.write().unwrap().1.take();
    //     let guardian = self.provider().root_guardian().clone();
    //     async fn stop(guardian: LocalActorRef) {
    //         let (tx, rx) = channel();
    //         guardian.cast(Shutdown { signal: tx }, ActorRef::no_sender());
    //         let _ = rx.await;
    //     }
    //     match rx {
    //         None => {
    //             tokio::select! {
    //                 _ = tokio::signal::ctrl_c() => {
    //                     stop(guardian).await;
    //                 }
    //             }
    //         }
    //         Some(rx) => {
    //             tokio::select! {
    //                 _ = tokio::signal::ctrl_c() => {
    //                     stop(guardian).await;
    //                 }
    //                 _ = rx => {
    //                     stop(guardian).await;
    //                 }
    //             }
    //         }
    //     }
    // }

    pub fn system_guardian(&self) -> LocalActorRef {
        self.provider().system_guardian().clone()
    }

    pub fn system_rt(&self) -> &Runtime {
        &self.system_rt
    }

    pub fn user_rt(&self) -> &Runtime {
        &self.user_rt
    }

    pub fn spawn_system(&self, mut props: Props, name: Option<String>) -> anyhow::Result<ActorRef> {
        props.handle = Some(self.system_rt.handle().clone());
        self.system_guardian().attach_child(props, name, true).map(|(actor, _)| actor)
    }

    pub fn event_stream(&self) -> &EventStream {
        &self.event_stream
    }

    pub fn scheduler(&self) -> &SchedulerSender {
        &self.scheduler
    }

    pub fn register_extension<E, F>(&self, ext_fn: F) where E: Extension, F: Fn(ActorSystem) -> E {
        let extension = ext_fn(self.clone());
        self.extensions.register(extension);
    }

    pub fn get_extension<E>(&self) -> Option<MappedRef<&'static str, Box<dyn Extension>, E>> where E: Extension {
        self.extensions.get()
    }

    pub fn get_extension_mut<E>(&self) -> Option<MappedRefMut<&'static str, Box<dyn Extension>, E>> where E: Extension {
        self.extensions.get_mut()
    }

    pub fn uid(&self) -> i64 {
        self.uid
    }

    pub fn core_config(&self) -> &CoreConfig {
        &self.core_config
    }

    fn build_runtime(name: &str) -> Runtime {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_name(name)
            .build()
            .expect("Failed building the Runtime");
        runtime
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
        self.guardian().attach_child(props, Some(name.into()), true).map(|(actor, _)| actor)
    }

    fn spawn_anonymous(&self, props: Props) -> anyhow::Result<ActorRef> {
        self.guardian().attach_child(props, None, true).map(|(actor, _)| actor)
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

#[pin_project]
pub struct ActorSystemRunner {
    pub system: ActorSystem,
    #[pin]
    signal: BoxFuture<'static, ()>,
}

impl Deref for ActorSystemRunner {
    type Target = ActorSystem;

    fn deref(&self) -> &Self::Target {
        &self.system
    }
}

impl Future for ActorSystemRunner {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let this = self.project();
        let _ = ready!(this.signal.poll(cx));
        Poll::Ready(())
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

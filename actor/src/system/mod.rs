use std::fmt::{Debug, Formatter};
use std::future::Future;
use std::ops::Deref;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use etcd_client::Client;
use futures::FutureExt;
use tokio::runtime::Runtime;

use crate::Actor;
use crate::actor_path::{ActorPath, TActorPath};
use crate::actor_ref::{ActorRef, ActorRefExt, TActorRef};
use crate::actor_ref::local_ref::LocalActorRef;
use crate::address::Address;
use crate::event::event_bus::SystemEventBus;
use crate::message::MessageRegistration;
use crate::props::Props;
use crate::provider::{ActorRefFactory, ActorRefProvider, TActorRefProvider};
use crate::provider::empty_provider::EmptyActorRefProvider;
use crate::provider::remote_provider::RemoteActorRefProvider;
use crate::system::config::{Config, EtcdConfig};
use crate::system::root_guardian::{AddShutdownHook, Shutdown};

pub mod root_guardian;
pub(crate) mod system_guardian;
pub(crate) mod user_guardian;
mod timer_scheduler;
pub mod config;

#[derive(Debug, Clone)]
pub struct ActorSystem {
    inner: Arc<Inner>,
}

pub struct Inner {
    address: Address,
    start_time: u128,
    provider: RwLock<ActorRefProvider>,
    reg: MessageRegistration,
    runtime: Runtime,
    client: Client,
}

impl Debug for Inner {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Inner")
            .field("address", &self.address)
            .field("start_time", &self.start_time)
            .field("provider", &self.provider)
            .field("reg", &self.reg)
            .field("runtime", &self.runtime)
            // .field("event_bus", &self.event_bus)
            .field("client", &"..")
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
        let inner = Inner {
            start_time: SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis(),
            address,
            provider: RwLock::new(EmptyActorRefProvider.into()),
            reg,
            runtime,
            client,
        };
        let system = Self {
            inner: inner.into(),
        };
        RemoteActorRefProvider::init(&system)?;
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
        self.guardian().stop();
    }

    pub async fn wait_termination(&self) {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                let (tx, mut rx) = tokio::sync::oneshot::channel();
                self.provider().root_guardian().cast_async(Shutdown { signal: tx }, ActorRef::no_sender());
                let _ = rx.await;
            }
        }
    }

    pub(crate) fn system_guardian(&self) -> LocalActorRef {
        self.provider().system_guardian().clone()
    }

    pub(crate) fn provider_rw(&self) -> &RwLock<ActorRefProvider> {
        &self.provider
    }

    pub(crate) fn registration(&self) -> &MessageRegistration {
        &self.reg
    }

    pub fn rt(&self) -> &Runtime {
        &self.runtime
    }

    pub fn spawn<F>(&self, future: F) -> tokio::task::JoinHandle<F::Output>
        where
            F: Future + Send + 'static,
            F::Output: Send + 'static, {
        self.rt().spawn(future)
    }

    pub(crate) fn system_actor_of<T>(&self, actor: T, arg: T::A, name: Option<String>, props: Props) -> anyhow::Result<ActorRef> where T: Actor {
        self.system_guardian().attach_child(actor, arg, name, props)
    }

    pub fn event_bus(&self) -> &SystemEventBus {
        // &self.event_bus
        todo!()
    }

    pub fn eclient(&self) -> &Client {
        &self.client
    }

    pub fn add_shutdown_hook<F>(&self, fut: F) where F: Future<Output=()> + Send + 'static {
        self.provider().root_guardian().cast(AddShutdownHook { fut: fut.boxed() }, ActorRef::no_sender());
    }
}

impl ActorRefFactory for ActorSystem {
    fn system(&self) -> &ActorSystem {
        &self
    }

    fn provider(&self) -> ActorRefProvider {
        self.provider.read().unwrap().clone()
    }

    fn guardian(&self) -> LocalActorRef {
        self.provider().guardian().clone()
    }

    fn lookup_root(&self) -> ActorRef {
        self.provider().root_guardian().clone().into()
    }

    fn actor_of<T>(
        &self,
        actor: T,
        arg: T::A,
        props: Props,
        name: Option<String>,
    ) -> anyhow::Result<ActorRef>
        where
            T: Actor,
    {
        self.guardian().attach_child(actor, arg, name, props)
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
    use crate::props::Props;
    use crate::provider::ActorRefFactory;
    use crate::system::ActorSystem;
    use crate::system::config::Config;

    #[tokio::test]
    async fn test_spawn_actor() -> anyhow::Result<()> {
        let system = ActorSystem::create(Config::default()).await?;
        for i in 0..10 {
            let name = format!("testActor{}", i);
            let actor = system.actor_of(EmptyTestActor, (), Props::default(), Some(name))?;
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

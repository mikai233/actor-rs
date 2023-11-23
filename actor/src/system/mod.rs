use std::future::Future;
use std::net::SocketAddrV4;
use std::ops::Deref;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::runtime::Runtime;

use crate::Actor;
use crate::actor_path::{ActorPath, TActorPath};
use crate::actor_ref::{ActorRef, ActorRefExt, TActorRef};
use crate::actor_ref::local_ref::LocalActorRef;
use crate::address::Address;
use crate::event::event_bus::ActorEventBus;
use crate::message::MessageRegistration;
use crate::props::Props;
use crate::provider::{ActorRefFactory, ActorRefProvider, TActorRefProvider};
use crate::provider::empty_provider::EmptyActorRefProvider;
use crate::provider::remote_provider::RemoteActorRefProvider;

pub mod root_guardian;
pub(crate) mod system_guardian;
pub(crate) mod user_guardian;
mod timer_scheduler;
mod config;

#[derive(Debug, Clone)]
pub struct ActorSystem {
    inner: Arc<Inner>,
}

#[derive(Debug)]
pub struct Inner {
    address: Address,
    start_time: u128,
    provider: RwLock<ActorRefProvider>,
    reg: MessageRegistration,
    runtime: Runtime,
    event_bus: ActorEventBus,
}

impl Deref for ActorSystem {
    type Target = Arc<Inner>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl ActorSystem {
    pub fn new(name: String, addr: SocketAddrV4, reg: MessageRegistration) -> anyhow::Result<Self> {
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
            event_bus: ActorEventBus::default(),
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

    fn start_time(&self) -> u128 {
        self.start_time
    }

    fn terminate(&self) {
        self.guardian().stop();
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

    pub(crate) fn rt(&self) -> &Runtime {
        &self.runtime
    }

    pub(crate) fn spawn<F>(&self, future: F) -> tokio::task::JoinHandle<F::Output>
        where
            F: Future + Send + 'static,
            F::Output: Send + 'static, {
        self.rt().spawn(future)
    }

    pub(crate) fn system_actor_of<T>(&self, actor: T, arg: T::A, name: Option<String>, props: Props) -> anyhow::Result<ActorRef> where T: Actor {
        self.system_guardian().attach_child(actor, arg, name, props)
    }

    pub fn event_bus(&self) -> &ActorEventBus {
        &self.event_bus
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
    use std::net::SocketAddrV4;
    use std::time::Duration;

    use tracing::info;

    use crate::{EmptyTestActor, EmptyTestMessage};
    use crate::actor_path::TActorPath;
    use crate::actor_ref::{ActorRef, ActorRefExt, TActorRef};
    use crate::message::MessageRegistration;
    use crate::props::Props;
    use crate::provider::ActorRefFactory;
    use crate::system::ActorSystem;

    #[tokio::test]
    async fn test_spawn_actor() -> anyhow::Result<()> {
        let name = "game".to_string();
        let addr: SocketAddrV4 = "127.0.0.1:12121".parse()?;
        let reg = MessageRegistration::new();
        let system = ActorSystem::new(name, addr, reg)?;
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

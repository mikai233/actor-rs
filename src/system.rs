use std::net::SocketAddrV4;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::anyhow;

use crate::actor::Actor;
use crate::actor::context::{ActorThreadPool, ActorThreadPoolMessage};
use crate::actor_path::{ActorPath, TActorPath};
use crate::actor_ref::{ActorRef, ActorRefExt, Cell, TActorRef};
use crate::actor_ref::local_ref::LocalActorRef;
use crate::address::Address;
use crate::cell::ActorCell;
use crate::cell::runtime::ActorRuntime;
use crate::ext::{check_name, random_actor_name};
use crate::net::mailbox::{Mailbox, MailboxSender};
use crate::props::Props;
use crate::provider::{ActorRefFactory, ActorRefProvider, TActorRefProvider};
use crate::provider::empty_provider::EmptyActorRefProvider;
use crate::provider::remote_provider::RemoteActorRefProvider;
use crate::{system_guardian, user_guardian};
use crate::user_guardian::UserGuardianMessage;

#[derive(Debug, Clone)]
pub struct ActorSystem {
    inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    address: Address,
    spawner: crossbeam::channel::Sender<ActorThreadPoolMessage>,
    pool: RwLock<ActorThreadPool>,
    start_time: u128,
    provider: RwLock<ActorRefProvider>,
}

impl ActorSystem {
    pub fn new(name: String, addr: SocketAddrV4) -> anyhow::Result<Self> {
        let pool = ActorThreadPool::new();
        let address = Address {
            protocol: "tcp".to_string(),
            system: name,
            addr,
        };
        let inner = Inner {
            start_time: SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis(),
            address,
            spawner: pool.sender.clone(),
            pool: pool.into(),
            provider: RwLock::new(EmptyActorRefProvider.into()),
        };
        let system = Self {
            inner: inner.into(),
        };
        RemoteActorRefProvider::init(&system)?;
        Ok(system)
    }
    pub fn name(&self) -> &String {
        &self.inner.address.system
    }

    pub fn address(&self) -> &Address {
        &&self.inner.address
    }

    fn child(&self, child: String) -> ActorPath {
        self.guardian().path.child(child)
    }

    fn descendant(&self, names: Vec<String>) -> ActorPath {
        self.guardian().path.descendant(names)
    }

    fn start_time(&self) -> u128 {
        self.inner.start_time
    }

    fn terminate(&self) {
        self.guardian().stop();
    }

    pub(crate) fn exec_actor_rt<T>(&self, rt: ActorRuntime<T>) -> anyhow::Result<()>
        where
            T: Actor,
    {
        if self.inner.spawner.send(rt.into()).is_err() {
            let name = std::any::type_name::<T>();
            return Err(anyhow!(
                "spawn actor {} error, actor thread pool shutdown",
                name
            ));
        }
        Ok(())
    }

    pub(crate) fn system_guardian(&self) -> LocalActorRef {
        self.provider().system_guardian().clone()
    }

    pub(crate) fn provider_rw(&self) -> &RwLock<ActorRefProvider> {
        &self.inner.provider
    }
}

impl ActorRefFactory for ActorSystem {
    fn system(&self) -> &ActorSystem {
        &self
    }

    fn provider(&self) -> ActorRefProvider {
        self.inner.provider.read().unwrap().clone()
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
        if let Some(name) = &name {
            check_name(name)?;
        }
        let name = name.unwrap_or_else(random_actor_name);
        let path = self.guardian().path.child(name);
        let rt = make_actor_runtime(
            self,
            actor,
            arg,
            props,
            path,
            Some(self.guardian().clone().into()),
        )?;
        let actor_ref = rt.myself.clone();
        self.exec_actor_rt(rt)?;
        Ok(actor_ref)
    }

    fn stop(&self, actor: &ActorRef) {
        let path = actor.path();
        let parent = path.parent();
        let guard = self.guardian();
        let sys = self.system_guardian();
        if parent == guard.path {
            guard.tell_local(
                user_guardian::StopChild {
                    child: actor.clone(),
                },
                None,
            );
        } else if parent == sys.path {
            sys.tell_local(
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

pub(crate) fn make_actor_runtime<T>(
    system: &ActorSystem,
    actor: T,
    arg: T::A,
    props: Props,
    path: ActorPath,
    parent: Option<ActorRef>,
) -> anyhow::Result<ActorRuntime<T>>
    where
        T: Actor,
{
    let name = path.name().clone();
    let (m_tx, m_rx) = tokio::sync::mpsc::channel(props.mailbox);
    let (s_tx, s_rx) = tokio::sync::mpsc::channel(1000);
    let sender = MailboxSender {
        message: m_tx,
        signal: s_tx,
    };
    let mailbox = Mailbox {
        message: m_rx,
        signal: s_rx,
    };
    let myself = LocalActorRef {
        system: system.clone(),
        path,
        sender,
        cell: ActorCell::new(parent),
    };
    if let Some(parent) = myself.parent().map(|a| a.local_or_panic()) {
        let mut children = parent.children().write().unwrap();
        if children.contains_key(&name) {
            return Err(anyhow!("duplicate actor name {}", name));
        }
        children.insert(name, myself.clone().into());
    }
    let rt = ActorRuntime {
        myself: myself.into(),
        handler: actor,
        props,
        system: system.clone(),
        mailbox,
        arg,
    };
    Ok(rt)
}

#[cfg(test)]
mod system_test {
    use std::net::SocketAddrV4;
    use std::time::Duration;
    use async_trait::async_trait;

    use tracing::info;

    use crate::actor::{Actor, Message};
    use crate::actor::context::ActorContext;
    use crate::actor_path::TActorPath;
    use crate::actor_ref::{ActorRefExt, TActorRef};
    use crate::props::Props;
    use crate::provider::ActorRefFactory;
    use crate::system::ActorSystem;

    #[derive(Debug)]
    pub struct TestActor;

    impl Actor for TestActor {
        type S = ();
        type A = ();

        fn pre_start(
            &self,
            _ctx: &mut ActorContext<Self>,
            _arg: Self::A,
        ) -> anyhow::Result<Self::S> {
            Ok(())
        }
    }

    #[async_trait(? Send)]
    impl Message for () {
        type T = TestActor;

        async fn handle(self: Box<Self>, context: &mut ActorContext<'_, Self::T>, state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_spawn_actor() -> anyhow::Result<()> {
        let name = "game".to_string();
        let addr: SocketAddrV4 = "127.0.0.1:12121".parse()?;
        let system = ActorSystem::new(name, addr)?;
        for i in 0..10 {
            let name = format!("testActor{}", i);
            let actor = system.actor_of(TestActor, (), Props::default(), Some(name))?;
            let elements: Vec<String> = actor.path().elements();
            info!("{:?}", elements);
            tokio::spawn(async move {
                info!("{}", actor);
                loop {
                    actor.tell_local((), None);
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            });
        }
        tokio::time::sleep(Duration::from_secs(10)).await;
        Ok(())
    }
}

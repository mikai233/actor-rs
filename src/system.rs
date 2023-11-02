use std::future::Future;
use std::net::SocketAddrV4;
use std::sync::{Arc, RwLock};

use anyhow::anyhow;
use futures::FutureExt;

use crate::actor::Actor;
use crate::actor::context::{ActorThreadPool, ActorThreadPoolMessage};
use crate::actor_path::{ActorPath, TActorPath};
use crate::actor_ref::{ActorRef, TActorRef};
use crate::actor_ref::local_ref::LocalActorRef;
use crate::address::Address;
use crate::cell::ActorCell;
use crate::cell::runtime::ActorRuntime;
use crate::ext::random_actor_name;
use crate::net::mailbox::{Mailbox, MailboxSender};
use crate::props::Props;
use crate::provider::{ActorRefFactory, ActorRefProvider, TActorRefProvider};
use crate::provider::local_provider::LocalActorRefProvider;
use crate::provider::remote_provider::RemoteActorRefProvider;

#[derive(Debug, Clone)]
pub struct ActorSystem {
    inner: Arc<Inner>,
    provider: Option<ActorRefProvider>,
}

#[derive(Debug)]
struct Inner {
    address: Address,
    spawner: crossbeam::channel::Sender<ActorThreadPoolMessage>,
    pool: RwLock<ActorThreadPool>,
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
            address,
            spawner: pool.sender.clone(),
            pool: pool.into(),
        };
        let mut system = Self {
            inner: inner.into(),
            provider: None,
        };
        let provider = RemoteActorRefProvider {
            local: LocalActorRefProvider::new(&system)?,
        };
        system.provider = Some(provider.into());
        Ok(system)
    }
    pub fn name(&self) -> &String {
        &self.inner.address.system
    }

    pub fn address(&self) -> &Address {
        &&self.inner.address
    }

    fn child(&self, child: String) -> ActorPath {
        todo!()
    }

    fn descendant(&self, names: Vec<String>) -> ActorPath {
        todo!()
    }

    fn start_time(&self) -> u64 {
        todo!()
    }

    fn terminate(&self) {
        todo!()
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
}

impl ActorRefFactory for ActorSystem {
    fn system(&self) -> &ActorSystem {
        &self
    }

    fn provider(&self) -> &ActorRefProvider {
        self.provider.as_ref().unwrap()
    }

    fn guardian(&self) -> &LocalActorRef {
        self.provider().guardian()
    }

    fn lookup_root(&self) -> ActorRef {
        todo!()
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
        let name = name.unwrap_or_else(random_actor_name);
        //TODO validate custom actor name
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
        todo!()
    }
}

#[derive(Debug)]
pub(crate) enum ActorParent {
    Other(ActorRef),
    Myself,
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
    if let Some(parent) = myself.parent() {
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

    use tracing::info;

    use crate::actor::Actor;
    use crate::actor::context::{ActorContext, Context};
    use crate::actor_path::TActorPath;
    use crate::actor_ref::{ActorRefExt, TActorRef};
    use crate::cell::envelope::UserEnvelope;
    use crate::props::Props;
    use crate::provider::ActorRefFactory;
    use crate::system::ActorSystem;

    #[derive(Debug)]
    struct TestActor;

    impl Actor for TestActor {
        type M = ();
        type S = ();
        type A = ();

        fn pre_start(&self, ctx: &mut ActorContext<Self>, arg: Self::A) -> anyhow::Result<Self::S> {
            Ok(())
        }

        fn on_recv(
            &self,
            ctx: &mut ActorContext<Self>,
            state: &mut Self::S,
            message: UserEnvelope<Self::M>,
        ) -> anyhow::Result<()> {
            info!("{} recv message", ctx.myself());
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

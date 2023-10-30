use std::future::Future;
use std::net::SocketAddrV4;
use std::pin::Pin;
use std::sync::{Arc, RwLock};

use anyhow::anyhow;
use futures::FutureExt;
use futures::stream::FuturesUnordered;

use crate::actor::Actor;
use crate::actor::context::{ActorThreadPool, ActorThreadPoolMessage};
use crate::actor_path::{ActorPath, RootActorPath, TActorPath};
use crate::actor_ref::ActorRef;
use crate::actor_ref::local_ref::LocalActorRef;
use crate::address::Address;
use crate::cell::runtime::ActorRuntime;
use crate::ext::random_actor_name;
use crate::net::mailbox::{Mailbox, MailboxSender};
use crate::props::Props;
use crate::provider::{ActorRefFactory, ActorRefProvider};

#[derive(Debug, Clone)]
pub struct ActorSystem {
    inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    address: Address,
    spawner: crossbeam::channel::Sender<ActorThreadPoolMessage>,
    pool: RwLock<ActorThreadPool>,
    provider: Option<ActorRefProvider>,
}

impl ActorSystem {
    fn new(name: String, addr: SocketAddrV4) -> Self {
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
            provider: None,
        };
        let system = Self {
            inner: inner.into(),
        };
        system
    }
    fn name(&self) -> &String {
        &self.inner.address.system
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
}

impl ActorRefFactory for ActorSystem {
    fn system(&self) -> &ActorSystem {
        &self
    }

    fn provider(&self) -> &ActorRefProvider {
        self.inner.provider.as_ref().unwrap()
    }

    fn guardian(&self) -> &ActorRef {
        todo!()
    }

    fn lookup_root(&self) -> ActorRef {
        todo!()
    }

    fn actor_of<T>(&self, actor: T, arg: T::A, props: Props, name: Option<String>) -> anyhow::Result<ActorRef> where T: Actor {
        let path = RootActorPath::new(self.inner.address.clone(), "/user".to_string());
        let actor_rt = make_actor_runtime::<T>(self, actor, arg, props, name, path.into())?;
        let actor_ref = actor_rt.actor_ref.clone();
        let spawn_fn = move |futures: &mut FuturesUnordered<Pin<Box<dyn Future<Output=()>>>>| {
            futures.push(actor_rt.run().boxed_local());
        };
        if self.inner.spawner.send(ActorThreadPoolMessage::SpawnActor(Box::new(spawn_fn))).is_err() {
            let name = std::any::type_name::<T>();
            return Err(anyhow!("spawn actor {} error, actor thread pool shutdown",name));
        }
        Ok(actor_ref)
    }

    fn stop(&self, actor: &ActorRef) {
        todo!()
    }
}

fn make_actor_runtime<T>(system: &ActorSystem, actor: T, arg: T::A, props: Props, name: Option<String>, parent: ActorPath) -> anyhow::Result<ActorRuntime<T>> where T: Actor {
    let name = match name {
        None => { random_actor_name() }
        Some(name) => { name }
    };
    let (m_tx, m_rx) = tokio::sync::mpsc::channel(props.mailbox);
    let (s_tx, s_rx) = tokio::sync::mpsc::channel(1000);
    let sender = MailboxSender { message: m_tx, signal: s_tx };
    let mailbox = Mailbox { message: m_rx, signal: s_rx };
    let actor_ref = LocalActorRef {
        system: system.clone(),
        path: parent.child(name),
        sender,
    };
    let rt = ActorRuntime {
        actor_ref: actor_ref.into(),
        handler: actor,
        props,
        system: system.clone(),
        parent: None,
        mailbox,
        arg,
    };
    Ok(rt)
}

#[cfg(test)]
mod system_test {
    use std::net::SocketAddrV4;

    use tracing::info;

    use crate::actor::Actor;
    use crate::actor::context::{ActorContext, Context};
    use crate::actor_ref::ActorRefExt;
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

        fn on_recv(&self, ctx: &mut ActorContext<Self>, state: &mut Self::S, message: Self::M) -> anyhow::Result<()> {
            info!("{} recv message",ctx.myself());
            Ok(())
        }
    }

    #[test]
    fn test_spawn_actor() -> anyhow::Result<()> {
        let name = "game".to_string();
        let addr: SocketAddrV4 = "127.0.0.1:12121".parse()?;
        let system = ActorSystem::new(name, addr);
        let actor = system.actor_of(TestActor, (), Props::default(), None)?;
        info!("{}", actor);
        actor.tell_local((), None);
        std::thread::park();
        Ok(())
    }
}
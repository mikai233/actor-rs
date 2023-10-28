use std::ops::Deref;
use std::sync::{Arc, Mutex, RwLock};
use crate::actor::Actor;
use crate::actor::context::ActorThreadPool;
use crate::actor_path::ActorPath;
use crate::actor_ref::ActorRef;
use crate::props::Props;
use crate::provider::{ActorRefFactory, ActorRefProvider};

#[derive(Debug, Clone)]
pub struct ActorSystem {
    inner: Arc<RwLock<Inner>>,
}

#[derive(Debug)]
struct Inner {
    name: String,
    pool: ActorThreadPool,
    provider: ActorRefProvider,
}

impl ActorSystem {
    fn new(name: String) -> Self {
        todo!()
        // let pool = ActorThreadPool::new();
        // let inner = Inner {
        //     name,
        //     pool,
        //     provider: (),
        // };
        // let system = Self {
        //     inner: Arc::new(RwLock::new(inner)),
        // };
        // system
    }
    fn name(&self) -> String {
        todo!()
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
        todo!()
    }

    fn provider(&self) -> &ActorRefProvider {
        todo!()
    }

    fn guardian(&self) -> &ActorRef {
        todo!()
    }

    fn lookup_root(&self) -> &ActorRef {
        todo!()
    }

    fn actor_of<T>(&self, actor: T, arg: T::A, props: Props, name: Option<String>) -> ActorRef where T: Actor {
        todo!()
    }

    fn stop(&self, actor: &ActorRef) {
        todo!()
    }
}
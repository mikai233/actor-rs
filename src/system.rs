use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, RwLock};

use futures::FutureExt;
use futures::stream::FuturesUnordered;
use tracing::error;

use crate::actor::Actor;
use crate::actor::context::{ActorThreadPool, ActorThreadPoolMessage};
use crate::actor_path::ActorPath;
use crate::actor_ref::ActorRef;
use crate::cell::runtime::ActorRuntime;
use crate::props::Props;
use crate::provider::{ActorRefFactory, ActorRefProvider};

#[derive(Debug, Clone)]
pub struct ActorSystem {
    name: String,
    pub(crate) spawner: crossbeam::channel::Sender<ActorThreadPoolMessage>,
    pool: Arc<RwLock<ActorThreadPool>>,
}

impl ActorSystem {
    fn new(name: String) -> Self {
        let pool = ActorThreadPool::new();
        let system = Self {
            name,
            spawner: pool.sender.clone(),
            pool: Arc::new(RwLock::new(pool)),
        };
        system
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
        self
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
        let actor_rt = make_actor_runtime::<T>(actor, arg);
        let actor_ref = actor_rt.actor_ref.clone();
        let spawn_fn = move |futures: &mut FuturesUnordered<Pin<Box<dyn Future<Output=()>>>>| {
            futures.push(actor_rt.run().boxed_local());
        };
        if self.spawner.send(ActorThreadPoolMessage::SpawnActor(Box::new(spawn_fn))).is_err() {
            let name = std::any::type_name::<T>();
            error!("spawn actor {} error, actor thread pool shutdown",name);
        }
        actor_ref
    }

    fn stop(&self, actor: &ActorRef) {
        todo!()
    }
}

fn make_actor_runtime<T>(actor: T, arg: T::A) -> ActorRuntime<T> where T: Actor {
    todo!()
}
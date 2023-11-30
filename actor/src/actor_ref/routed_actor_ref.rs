use std::fmt::{Debug, Formatter};
use std::ops::Deref;
use std::sync::Arc;

use arc_swap::ArcSwap;

use crate::actor_path::ActorPath;
use crate::actor_ref::{ActorRef, TActorRef};
use crate::DynMessage;
use crate::routing::router::Router;
use crate::system::ActorSystem;

#[derive(Clone)]
pub struct RoutedActorRef {
    pub(crate) inner: Arc<Inner>,
}

pub struct Inner {
    pub(crate) system: ActorSystem,
    pub(crate) path: ActorPath,
    pub(crate) router: ArcSwap<Router>,
}

impl Deref for RoutedActorRef {
    type Target = Arc<Inner>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Debug for RoutedActorRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RemoteActorRef")
            .field("system", &"..")
            .field("path", &self.path)
            .finish()
    }
}

impl TActorRef for RoutedActorRef {
    fn system(&self) -> ActorSystem {
        self.system.clone()
    }

    fn path(&self) -> &ActorPath {
        &self.path
    }

    fn tell(&self, message: DynMessage, sender: Option<ActorRef>) {
        todo!()
    }

    fn stop(&self) {
        todo!()
    }

    fn parent(&self) -> Option<&ActorRef> {
        todo!()
    }

    fn get_child<I>(&self, names: I) -> Option<ActorRef> where I: IntoIterator<Item=String> {
        todo!()
    }
}
use std::fmt::{Debug, Formatter};
use std::ops::Deref;
use std::sync::Arc;

use arc_swap::ArcSwap;
use crate::actor::actor_path::ActorPath;

use crate::actor::actor_ref::{ActorRef, TActorRef};
use crate::actor::actor_system::ActorSystem;
use crate::DynMessage;
use crate::props::Props;
use crate::routing::router::Router;
use crate::routing::router_config::TRouterConfig;

#[derive(Clone)]
pub struct RoutedActorRef {
    pub(crate) inner: Arc<Inner>,
}

pub struct Inner {
    pub(crate) system: ActorSystem,
    pub(crate) path: ActorPath,
    pub(crate) router_actor: ActorRef,
    pub(crate) router: ArcSwap<Router>,
    pub(crate) router_props: Props,
    pub(crate) parent: ActorRef,
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
        match self.router_props.router_config().unwrap().is_management_message(&message) {
            true => {
                self.router_actor.tell(message, sender);
            }
            false => {
                self.router.load().route(message, sender);
            }
        }
    }

    fn stop(&self) {
        self.router_actor.stop()
    }

    fn parent(&self) -> Option<&ActorRef> {
        todo!()
    }

    fn get_child(&self, names: Vec<String>) -> Option<ActorRef> {
        self.router_actor.get_child(names)
    }
}
use std::collections::BTreeMap;
use std::fmt::Display;
use std::sync::{Arc, RwLock};

use crate::actor_path::ActorPath;
use crate::actor_ref::{ActorRef, TActorRef};
use crate::message::ActorMessage;
use crate::net::tcp_transport::TcpTransport;
use crate::system::ActorSystem;

#[derive(Debug, Clone)]
pub struct RemoteActorRef {
    system: ActorSystem,
    transport: Arc<TcpTransport>,
    path: ActorPath,
}

impl TActorRef for RemoteActorRef {
    fn system(&self) -> ActorSystem {
        self.system.clone()
    }

    fn path(&self) -> &ActorPath {
        todo!()
    }

    fn tell(&self, message: ActorMessage, sender: Option<ActorRef>) {
        todo!()
    }

    fn start(&self) {
        todo!()
    }

    fn stop(&self) {
        todo!()
    }

    fn parent(&self) -> Option<&ActorRef> {
        todo!()
    }

    fn children(&self) -> &RwLock<BTreeMap<String, ActorRef>> {
        todo!()
    }
}
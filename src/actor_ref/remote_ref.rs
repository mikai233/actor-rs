use std::fmt::Display;
use std::sync::Arc;

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
}
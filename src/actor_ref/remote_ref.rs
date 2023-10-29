use crate::actor_path::TActorPath;
use std::fmt::{Display, Formatter};
use std::sync::Arc;

use crate::actor_path::ActorPath;
use crate::actor_ref::{ActorRef, TActorRef};
use crate::message::ActorMessage;
use crate::net::tcp_transport::TcpTransport;

#[derive(Debug, Clone)]
pub struct RemoteActorRef {
    transport: Arc<TcpTransport>,
    path: ActorPath,
}

impl Display for RemoteActorRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let path = self.path();
        let uid = path.uid();
        write!(f, "Actor[{}#{}]", path, uid)
    }
}

impl TActorRef for RemoteActorRef {
    fn path(&self) -> &ActorPath {
        todo!()
    }

    fn tell(&self, message: ActorMessage, sender: Option<ActorRef>) {
        todo!()
    }
}
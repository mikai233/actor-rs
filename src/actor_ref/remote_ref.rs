use std::sync::Arc;

use tracing::warn;

use crate::actor_path::ActorPath;
use crate::actor_ref::{ActorRef, ActorRefExt, TActorRef};
use crate::message::ActorMessage;
use crate::net::message::RemoteEnvelope;
use crate::net::tcp_transport::TransportMessage;
use crate::system::ActorSystem;

#[derive(Debug, Clone)]
pub struct RemoteActorRef {
    pub(crate) system: ActorSystem,
    pub(crate) path: ActorPath,
    pub(crate) transport: Arc<ActorRef>,
}

impl TActorRef for RemoteActorRef {
    fn system(&self) -> ActorSystem {
        self.system.clone()
    }

    fn path(&self) -> &ActorPath {
        &self.path
    }

    fn tell(&self, message: ActorMessage, sender: Option<ActorRef>) {
        match message {
            ActorMessage::Local(_) => {
                warn!("local message to remote actor ref");
            }
            ActorMessage::Remote(r) => {
                let envelope = RemoteEnvelope {
                    message: r,
                    sender,
                    target: self.clone().into(),
                };
                self.transport.tell_local(TransportMessage::OutboundMessage(envelope), None);
            }
        }
    }

    fn stop(&self) {
        todo!()
    }

    fn parent(&self) -> Option<&ActorRef> {
        None
    }

    fn get_child<I>(&self, names: I) -> Option<ActorRef> where I: IntoIterator<Item=String> {
        todo!()
    }
}

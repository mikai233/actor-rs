use serde::{Deserialize, Serialize};

use crate::actor_ref::{ActorRef, SerializedActorRef};
use crate::message::ActorRemoteMessage;

pub(crate) struct RemoteEnvelope {
    pub(crate) message: ActorRemoteMessage,
    pub(crate) sender: Option<ActorRef>,
    pub(crate) target: ActorRef,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct RemotePacket {
    pub(crate) message: ActorRemoteMessage,
    pub(crate) sender: Option<SerializedActorRef>,
    pub(crate) target: SerializedActorRef,
}

impl Into<RemotePacket> for RemoteEnvelope {
    fn into(self) -> RemotePacket {
        RemotePacket {
            message: self.message,
            sender: self.sender.map(|s| s.into()),
            target: self.target.into(),
        }
    }
}
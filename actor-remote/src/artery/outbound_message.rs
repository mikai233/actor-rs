use actor_core::actor::behavior::Behavior;
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::actor_path::TActorPath;
use actor_core::message::handler::MessageHandler;
use actor_core::message::DynMessage;
use actor_core::Message;
use anyhow::anyhow;
use tokio::sync::mpsc::error::TrySendError;
use tracing::{debug, warn};

use actor_core::actor_ref::{ActorRef, ActorRefExt};

use crate::artery::connect_tcp::ConnectTcp;
use crate::artery::connection_status::ConnectionStatus;
use crate::artery::disconnect::Disconnect;
use crate::artery::ArteryActor;

#[derive(Debug, Message, derive_more::Display, derive_more::Constructor)]
#[display("OutboundMessage {{ message: {message}, sender: {sender:?}, target: {target} }}")]
pub(crate) struct OutboundMessage {
    pub(crate) message: DynMessage,
    pub(crate) sender: Option<ActorRef>,
    pub(crate) target: ActorRef,
}

impl MessageHandler<ArteryActor> for OutboundMessage {
    fn handle(
        actor: &mut ArteryActor,
        ctx: &mut <ArteryActor as Actor>::Context,
        message: Self,
        sender: Option<ActorRef>,
        _: &Receive<ArteryActor>,
    ) -> anyhow::Result<Behavior<ArteryActor>> {
        let target = &message.target;
        let target_addr = target
            .path()
            .address()
            .addr
            .ok_or_else(|| anyhow!("no address in target actor path {}", target.path()))?;
        let target_connection = actor
            .connections
            .entry(target_addr)
            .or_insert(ConnectionStatus::NotConnected);
        match target_connection {
            ConnectionStatus::NotConnected => {
                debug!(
                    "connection to `{}` not established, stash `{}` and start connect",
                    target_addr,
                    message.message.signature()
                );
                ctx.myself()
                    .cast_ns(ConnectTcp::with_infinite_retry(target_addr));
                ArteryActor::buffer_message(
                    &mut actor.message_buffer,
                    target_addr,
                    message,
                    sender,
                    actor.advanced.outbound_message_buffer,
                );
                *target_connection = ConnectionStatus::PrepareForConnect;
            }
            ConnectionStatus::PrepareForConnect | ConnectionStatus::Connecting(_) => {
                debug!(
                    "connection to `{}` is establishing, stash `{}` and wait it established",
                    target_addr,
                    message.message.signature()
                );
                ArteryActor::buffer_message(
                    &mut actor.message_buffer,
                    target_addr,
                    message,
                    sender,
                    actor.advanced.outbound_message_buffer,
                );
            }
            ConnectionStatus::Connected(tx) => {
                if let Some(error) = tx.try_send(message).err() {
                    match error {
                        TrySendError::Full(msg) => {
                            warn!(
                                "message `{}` to `{}` connection buffer full, current message dropped",
                                msg.message.signature(), target_addr
                            );
                        }
                        TrySendError::Closed(msg) => {
                            ctx.myself().cast_ns(Disconnect { addr: target_addr });
                            warn!(
                                "message `{}` to `{}` connection closed",
                                msg.message.signature(),
                                target_addr
                            );
                        }
                    }
                }
            }
        }
        Ok(Behavior::same())
    }
}

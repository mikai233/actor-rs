use std::net::SocketAddr;

use actor_core::actor::behavior::Behavior;
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::message::handler::MessageHandler;
use actor_core::Message;
use anyhow::anyhow;
use tokio::sync::mpsc::error::TrySendError;
use tracing::{debug, warn};

use actor_core::actor::context::ActorContext;
use actor_core::actor_path::TActorPath;
use actor_core::actor_ref::{ActorRef, ActorRefExt};

use crate::artery::connect_tcp::ConnectTcp;
use crate::artery::connection_status::ConnectionStatus;
use crate::artery::disconnect::Disconnect;
use crate::artery::remote_envelope::RemoteEnvelope;
use crate::artery::ArteryActor;

#[derive(Debug, Message, derive_more::Display)]
#[display("OutboundMessage {{ name: {name}, envelope: {envelope} }}")]
pub(crate) struct OutboundMessage {
    pub(crate) name: &'static str,
    pub(crate) envelope: RemoteEnvelope,
}

impl MessageHandler<ArteryActor> for OutboundMessage {
    fn handle(
        actor: &mut ArteryActor,
        ctx: &mut <ArteryActor as Actor>::Context,
        message: Self,
        sender: Option<ActorRef>,
        _: &Receive<ArteryActor>,
    ) -> anyhow::Result<Behavior<ArteryActor>> {
        let addr: SocketAddr = message
            .envelope
            .target
            .path()
            .address()
            .addr
            .map(|a| a.into())
            .ok_or(anyhow!("socket addr not set"))?;
        let sender = actor
            .connections
            .entry(addr)
            .or_insert(ConnectionStatus::NotConnected);
        match sender {
            ConnectionStatus::NotConnected => {
                debug!(
                    "connection to {} not established, stash {} and start connect",
                    addr, message.name
                );
                ctx.myself().cast_ns(ConnectTcp::with_infinite_retry(addr));
                ArteryActor::buffer_message(
                    &mut actor.message_buffer,
                    addr,
                    message,
                    sender,
                    actor.advanced.outbound_message_buffer,
                );
                *sender = ConnectionStatus::PrepareForConnect;
            }
            ConnectionStatus::PrepareForConnect | ConnectionStatus::Connecting(_) => {
                debug!(
                    "connection to {} is establishing, stash {} and wait it established",
                    addr, message.name
                );
                ArteryActor::buffer_message(
                    &mut actor.message_buffer,
                    addr,
                    message,
                    sender,
                    actor.advanced.outbound_message_buffer,
                );
            }
            ConnectionStatus::Connected(tx) => {
                if let Some(error) = tx.try_send(message.envelope).err() {
                    match error {
                        TrySendError::Full(_) => {
                            warn!(
                                "message {} to {} connection buffer full, current message dropped",
                                message.name, addr
                            );
                        }
                        TrySendError::Closed(_) => {
                            ctx.myself().cast_ns(Disconnect { addr });
                            warn!("message {} to {} connection closed", message.name, addr);
                        }
                    }
                }
            }
        }
        Ok(Behavior::same())
    }
}

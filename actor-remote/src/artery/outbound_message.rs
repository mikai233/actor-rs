use std::net::SocketAddr;

use anyhow::anyhow;
use async_trait::async_trait;
use tokio::sync::mpsc::error::TrySendError;
use tracing::{debug, warn};

use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor_path::TActorPath;
use actor_core::actor_ref::ActorRefExt;
use actor_core::EmptyCodec;
use actor_core::ext::option_ext::OptionExt;
use actor_core::Message;

use crate::artery::ArteryActor;
use crate::artery::connect_tcp::ConnectTcp;
use crate::artery::connection_status::ConnectionStatus;
use crate::artery::disconnect::Disconnect;
use crate::artery::remote_envelope::RemoteEnvelope;
use crate::config::artery::Transport;

#[derive(Debug, EmptyCodec)]
pub(crate) struct OutboundMessage {
    pub(crate) name: &'static str,
    pub(crate) envelope: RemoteEnvelope,
}

#[async_trait]
impl Message for OutboundMessage {
    type A = ArteryActor;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let addr: SocketAddr = self.envelope.target.path()
            .address().addr
            .map(|a| a.into())
            .ok_or(anyhow!("socket addr not set"))?;
        let sender = actor.connections.entry(addr).or_insert(ConnectionStatus::NotConnected);
        match sender {
            ConnectionStatus::NotConnected => {
                debug!("connection to {} not established, stash {} and start connect", addr, self.name);
                match actor.transport {
                    Transport::Tcp => {
                        context.myself().cast_ns(ConnectTcp::with_infinite_retry(addr));
                    }
                    Transport::TlsTcp => {
                        unimplemented!("tls tcp unimplemented");
                    }
                }
                Self::A::buffer_message(&mut actor.message_buffer, addr, *self, context.sender().cloned(), actor.advanced.outbound_message_buffer);
                *sender = ConnectionStatus::PrepareForConnect;
            }
            ConnectionStatus::PrepareForConnect | ConnectionStatus::Connecting(_) => {
                debug!("connection to {} is establishing, stash {} and wait it established", addr, self.name);
                Self::A::buffer_message(&mut actor.message_buffer, addr, *self, context.sender().cloned(), actor.advanced.outbound_message_buffer);
            }
            ConnectionStatus::Connected(tx) => {
                if let Some(error) = tx.try_send(self.envelope).err() {
                    match error {
                        TrySendError::Full(_) => {
                            warn!("message {} to {} connection buffer full, current message dropped", self.name, addr);
                        }
                        TrySendError::Closed(_) => {
                            context.myself().cast(Disconnect { addr }, None);
                            warn!( "message {} to {} connection closed", self.name, addr );
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
use std::net::SocketAddr;

use eyre::anyhow;
use async_trait::async_trait;
use tokio::sync::mpsc::error::TrySendError;
use tracing::{debug, warn};

use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor_path::TActorPath;
use actor_core::actor_ref::ActorRefExt;
use actor_core::Message;
use actor_derive::EmptyCodec;

use crate::net::remote_envelope::RemoteEnvelope;
use crate::net::tcp_transport::connect::Connect;
use crate::net::tcp_transport::connection_status::ConnectionStatus;
use crate::net::tcp_transport::disconnect::Disconnect;
use crate::net::tcp_transport::TcpTransportActor;

#[derive(Debug, EmptyCodec)]
pub(crate) struct OutboundMessage {
    pub(crate) name: &'static str,
    pub(crate) envelope: RemoteEnvelope,
}

#[async_trait]
impl Message for OutboundMessage {
    type A = TcpTransportActor;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> eyre::Result<()> {
        let addr: SocketAddr = self.envelope.target.path()
            .address().addr
            .map(|a| a.into())
            .ok_or(anyhow!("socket addr not set"))?;
        let sender = actor.connections.entry(addr).or_insert(ConnectionStatus::NotConnected);
        match sender {
            ConnectionStatus::NotConnected => {
                debug!("connection to {} not established, stash {} and start connect", addr, self.name);
                context.myself().cast_ns(Connect::with_infinite_retry(addr));
                Self::A::buffer_message(&mut actor.message_buffer, &actor.transport, addr, *self, context.sender().cloned());
                *sender = ConnectionStatus::PrepareForConnect;
            }
            ConnectionStatus::PrepareForConnect | ConnectionStatus::Connecting(_) => {
                debug!("connection to {} is establishing, stash {} and wait it established", addr, self.name);
                Self::A::buffer_message(&mut actor.message_buffer, &actor.transport, addr, *self, context.sender().cloned());
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
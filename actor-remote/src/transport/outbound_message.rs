use std::any::type_name;
use std::net::SocketAddr;

use anyhow::anyhow;
use async_trait::async_trait;
use tokio::sync::mpsc::error::TrySendError;
use tracing::{debug, warn};

use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor_path::TActorPath;
use actor_core::actor_ref::ActorRefExt;
use actor_core::ext::option_ext::OptionExt;
use actor_core::Message;
use actor_derive::EmptyCodec;

use crate::config::transport::Transport;
use crate::transport::connect_quic::ConnectQuic;
use crate::transport::connect_tcp::ConnectTcp;
use crate::transport::connection_status::ConnectionStatus;
use crate::transport::disconnect::Disconnect;
use crate::transport::remote_envelope::RemoteEnvelope;
use crate::transport::TransportActor;

#[derive(Debug, EmptyCodec)]
pub(crate) struct OutboundMessage {
    pub(crate) name: &'static str,
    pub(crate) envelope: RemoteEnvelope,
}

#[async_trait]
impl Message for OutboundMessage {
    type A = TransportActor;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let addr: SocketAddr = self.envelope.target.path()
            .address().addr
            .map(|a| a.into())
            .ok_or(anyhow!("socket addr not set"))?;
        let sender = actor.connections.entry(addr).or_insert(ConnectionStatus::NotConnected);
        match sender {
            ConnectionStatus::NotConnected => {
                debug!("connection to {} not established, stash {} and start connect", addr, self.name);
                match &actor.transport {
                    Transport::Tcp(_) => {
                        context.myself().cast_ns(ConnectTcp::with_infinite_retry(addr));
                    }
                    Transport::Kcp(_) => {
                        unimplemented!("kcp unimplemented");
                    }
                    Transport::Quic(transport) => {
                        let (_, server_cert) = transport.config.as_result()?;
                        let config = TransportActor::configure_client(&[server_cert])?;
                        let connect_quic = ConnectQuic { addr, config };
                        context.myself().cast_ns(connect_quic);
                    }
                }
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
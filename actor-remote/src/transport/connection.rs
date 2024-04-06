use std::net::SocketAddr;

use futures::SinkExt;
use tokio::io::AsyncWrite;
use tokio_util::codec::FramedWrite;
use tracing::warn;

use actor_core::actor::address::Address;
use actor_core::actor_ref::{ActorRef, ActorRefExt};
use actor_core::ext::encode_bytes;

use crate::transport::codec::{Packet, PacketCodec};
use crate::transport::disconnect::Disconnect;
use crate::transport::disconnected::Disconnected;
use crate::transport::remote_envelope::RemoteEnvelope;
use crate::transport::remote_packet::RemotePacket;

pub type ConnectionTx = tokio::sync::mpsc::Sender<RemoteEnvelope>;
pub type ConnectionRx = tokio::sync::mpsc::Receiver<RemoteEnvelope>;

#[derive(Debug)]
pub(super) struct Connection<T: AsyncWrite + Unpin + Send + 'static> {
    pub(super) peer: SocketAddr,
    pub(super) myself: Address,
    pub(super) framed: FramedWrite<T, PacketCodec>,
    pub(super) rx: ConnectionRx,
    pub(super) transport: ActorRef,
}

impl<T> Connection<T> where T: AsyncWrite + Unpin + Send + 'static {
    pub fn new(
        addr: SocketAddr,
        framed: FramedWrite<T, PacketCodec>,
        transport: ActorRef,
        myself: Address,
    ) -> (Self, ConnectionTx) {
        let (tx, rx) = tokio::sync::mpsc::channel(10000);
        let myself = Self {
            peer: addr,
            myself,
            framed,
            rx,
            transport,
        };
        (myself, tx)
    }
    pub fn start(self) {
        let mut connection = self;
        tokio::spawn(async move {
            loop {
                match connection.rx.recv().await {
                    None => {
                        connection.disconnected();
                        break;
                    }
                    Some(envelope) => {
                        if let Some(err) = connection.send(envelope).await.err() {
                            connection.disconnect();
                            let addr = &connection.peer;
                            warn!("send message to {} error {:?}, drop current connection", addr, err);
                        }
                    }
                }
            }
        });
    }

    fn disconnect(&self) {
        self.transport.cast(Disconnect { addr: self.peer }, None);
    }

    fn disconnected(&self) {
        self.transport.cast(Disconnected { addr: self.peer }, None);
    }

    async fn send(&mut self, envelope: RemoteEnvelope) -> eyre::Result<()> {
        let packet: RemotePacket = envelope.into();
        let bytes = encode_bytes(&packet)?;
        self.framed.send(Packet::new(bytes)).await?;
        Ok(())
    }
}
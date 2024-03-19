use std::net::SocketAddr;

use futures::SinkExt;
use stubborn_io::tokio::StubbornIo;
use tokio::net::TcpStream;
use tokio_util::codec::Framed;
use tracing::warn;

use actor_core::actor::address::Address;
use actor_core::actor_ref::{ActorRef, ActorRefExt};
use actor_core::ext::encode_bytes;

use crate::net::codec::{Packet, PacketCodec};
use crate::net::remote_envelope::RemoteEnvelope;
use crate::net::remote_packet::RemotePacket;
use crate::net::tcp_transport::disconnect::Disconnect;
use crate::net::tcp_transport::disconnected::Disconnected;

pub type ConnectionTx = tokio::sync::mpsc::Sender<RemoteEnvelope>;
pub type ConnectionRx = tokio::sync::mpsc::Receiver<RemoteEnvelope>;

pub(super) struct Connection {
    pub(super) peer: SocketAddr,
    pub(super) myself: Address,
    pub(super) framed: Framed<StubbornIo<TcpStream, SocketAddr>, PacketCodec>,
    pub(super) rx: ConnectionRx,
    pub(super) transport: ActorRef,
}

impl Connection {
    pub fn new(
        addr: SocketAddr,
        framed: Framed<StubbornIo<TcpStream, SocketAddr>, PacketCodec>,
        transport: ActorRef,
    ) -> (Self, ConnectionTx) {
        let (tx, rx) = tokio::sync::mpsc::channel(10000);
        let myself = transport.system().address();
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

    async fn send(&mut self, envelope: RemoteEnvelope) -> anyhow::Result<()> {
        let packet: RemotePacket = envelope.into();
        let bytes = encode_bytes(&packet)?;
        self.framed.send(Packet::new(bytes)).await?;
        Ok(())
    }
}
use std::fmt::Debug;
use std::net::SocketAddr;
use futures::SinkExt;
use stubborn_io::tokio::StubbornIo;
use tokio::net::TcpStream;
use tokio_util::codec::Framed;
use tracing::{debug, warn};
use crate::actor_ref::{ActorRef, ActorRefExt};
use crate::ext::encode_bytes;
use crate::net::codec::{Packet, PacketCodec};
use crate::net::message::{RemoteEnvelope, RemotePacket};
use crate::net::tcp_transport::TransportMessage;

pub(crate) type ConnectionTx = tokio::sync::mpsc::Sender<RemoteEnvelope>;
pub(crate) type ConnectionRx = tokio::sync::mpsc::Receiver<RemoteEnvelope>;

pub(crate) struct Connection {
    pub(crate) addr: SocketAddr,
    pub(crate) framed: Framed<StubbornIo<TcpStream, SocketAddr>, PacketCodec>,
    pub(crate) rx: tokio::sync::mpsc::Receiver<RemoteEnvelope>,
    pub(crate) transport: ActorRef,
}

impl Connection {
    pub(crate) fn new(addr: SocketAddr, framed: Framed<StubbornIo<TcpStream, SocketAddr, >, PacketCodec>, transport: ActorRef) -> (Self, ConnectionTx) {
        let (tx, rx) = tokio::sync::mpsc::channel(10000);
        let myself = Self {
            addr,
            framed,
            rx,
            transport,
        };
        (myself, tx)
    }
    pub(crate) fn start(self) {
        let mut connection = self;
        tokio::spawn(async move {
            loop {
                match connection.rx.recv().await {
                    None => {
                        connection.disconnect();
                        break;
                    }
                    Some(envelope) => {
                        if let Some(err) = connection.send(envelope).await.err() {
                            connection.disconnect();
                            let addr = &connection.addr;
                            warn!("send message to {} error {:?}, drop current connection", addr, err);
                        }
                    }
                }
            }
        });
    }
    fn disconnect(&self) {
        self.transport.tell_local(TransportMessage::Disconnect(self.addr), None);
    }

    async fn send(&mut self, remote_envelope: RemoteEnvelope) -> anyhow::Result<()> {
        let packet: RemotePacket = remote_envelope.into();
        let bytes = encode_bytes(&packet)?;
        self.framed.send(Packet::new(bytes)).await?;
        Ok(())
    }
}
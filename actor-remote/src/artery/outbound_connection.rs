use std::net::SocketAddr;
use std::sync::Arc;

use actor_core::actor::actor_system::ActorSystem;
use actor_core::actor_path::TActorPath;
use futures::SinkExt;
use tokio::io::AsyncWrite;
use tokio_util::codec::FramedWrite;
use tracing::warn;

use actor_core::actor_ref::{ActorRef, ActorRefExt};

use crate::artery::codec::PacketCodec;
use crate::artery::disconnected::Disconnected;
use crate::artery::message_packet::MessagePacket;
use crate::codec::MessageCodecRegistry;

use super::codec::Packet;
use super::outbound_message::OutboundMessage;

pub(crate) type ConnectionTx = tokio::sync::mpsc::Sender<OutboundMessage>;
pub(crate) type ConnectionRx = tokio::sync::mpsc::Receiver<OutboundMessage>;

#[derive(Debug)]
pub(super) struct OutboundConnection<T: AsyncWrite + Unpin + Send + 'static> {
    pub(super) peer_addr: SocketAddr,
    pub(super) framed: FramedWrite<T, PacketCodec>,
    pub(super) rx: ConnectionRx,
    pub(super) artery: ActorRef,
    pub(super) system: ActorSystem,
    pub(super) registry: Arc<dyn MessageCodecRegistry>,
}

impl<T> OutboundConnection<T>
where
    T: AsyncWrite + Unpin + Send + 'static,
{
    pub(crate) fn new(
        peer_addr: SocketAddr,
        framed: FramedWrite<T, PacketCodec>,
        artery: ActorRef,
        system: ActorSystem,
        registry: Arc<dyn MessageCodecRegistry>,
    ) -> (Self, ConnectionTx) {
        let (tx, rx) = tokio::sync::mpsc::channel(10000);
        let myself = Self {
            peer_addr,
            framed,
            rx,
            artery,
            system,
            registry,
        };
        (myself, tx)
    }

    pub(crate) async fn receive_loop(mut self) {
        loop {
            match self.rx.recv().await {
                None => {
                    self.disconnected();
                    break;
                }
                Some(msg) => {
                    let signature = msg.message.signature();
                    match self.encode(msg) {
                        Ok(packet) => {
                            if let Some(err) = self.framed.send(packet).await.err() {
                                self.disconnected();
                                let addr = &self.peer_addr;
                                warn!(
                                    "send message to {} error {:?}, drop current connection",
                                    addr, err
                                );
                                break;
                            }
                        }
                        Err(encode_err) => {
                            warn!("encode message {} error {:?}", signature, encode_err);
                        }
                    };
                }
            }
        }
    }

    fn encode(&self, message: OutboundMessage) -> anyhow::Result<Packet> {
        let OutboundMessage {
            message,
            sender,
            target,
        } = message;
        let message_bytes = self.registry.encode(&*message, &self.system)?;
        let sender = sender.map(|s| s.path().to_serialization_format());
        let target = target.path().to_serialization_format();
        let message_packet = MessagePacket::new(message_bytes, sender, target);
        let bytes = bincode::serialize(&message_packet)?;
        Ok(Packet(bytes))
    }

    fn disconnected(&self) {
        self.artery.cast_ns(Disconnected {
            addr: self.peer_addr,
        });
    }
}

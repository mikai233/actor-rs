use std::{net::SocketAddr, sync::Arc};

use actor_core::{
    actor::actor_system::ActorSystem,
    actor_ref::{ActorRef, ActorRefExt},
};
use futures::StreamExt;
use tokio::io::AsyncRead;
use tokio_util::codec::FramedRead;
use tracing::warn;

use crate::codec::MessageCodecRegistry;

use super::{
    codec::{Packet, PacketCodec},
    inbound_message::InboundMessage,
    message_packet::MessagePacket,
};

#[derive(Debug)]
pub(super) struct InboundConnection<T: AsyncRead + Unpin + Send + 'static> {
    pub(super) peer_addr: SocketAddr,
    pub(super) framed: FramedRead<T, PacketCodec>,
    pub(super) artery: ActorRef,
    pub(super) system: ActorSystem,
    pub(super) registry: Arc<dyn MessageCodecRegistry>,
}

impl<T> InboundConnection<T>
where
    T: AsyncRead + Unpin + Send + 'static,
{
    pub(crate) fn new(
        peer_addr: SocketAddr,
        framed: FramedRead<T, PacketCodec>,
        artery: ActorRef,
        system: ActorSystem,
        registry: Arc<dyn MessageCodecRegistry>,
    ) -> Self {
        Self {
            peer_addr,
            framed,
            artery,
            system,
            registry,
        }
    }

    pub(crate) async fn receive_loop(mut self) {
        loop {
            match self.framed.next().await {
                Some(Ok(packet)) => match self.decode(packet) {
                    Ok(message) => {
                        self.artery.cast_ns(message);
                    }
                    Err(error) => {
                        warn!("deserialize {} message error {:?}", self.peer_addr, error);
                    }
                },
                Some(Err(error)) => {
                    warn!(
                        "decode {} packet error {:?}, drop current connection",
                        self.peer_addr, error
                    );
                    break;
                }
                None => {
                    break;
                }
            }
        }
    }

    pub(crate) fn decode(&self, packet: Packet) -> anyhow::Result<InboundMessage> {
        let MessagePacket {
            message_bytes,
            sender,
            target,
        } = bincode::deserialize(&packet)?;
        let message = self.registry.decode(&message_bytes, &self.system)?;
        let message = InboundMessage::new(message, sender, target);
        Ok(message)
    }
}

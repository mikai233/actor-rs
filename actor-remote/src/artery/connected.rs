use std::net::SocketAddr;

use actor_core::actor::behavior::Behavior;
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::actor_ref::ActorRef;
use actor_core::message::handler::MessageHandler;
use actor_core::Message;
use stubborn_io::tokio::StubbornIo;
use tokio_util::codec::FramedWrite;
use tracing::info;

use actor_core::message::message_buffer::BufferEnvelope;

use crate::artery::codec::PacketCodec;
use crate::artery::connection_status::ConnectionStatus;
use crate::artery::outbound_connection::OutboundConnection;
use crate::artery::ArteryActor;

#[derive(derive_more::Debug, Message, derive_more::Display, derive_more::Constructor)]
#[display("Connected({peer_addr})")]
pub(super) struct Connected {
    #[debug(skip)]
    pub(super) stream: StubbornIo<tokio::net::TcpStream, SocketAddr>,
    pub(super) peer_addr: SocketAddr,
}

impl MessageHandler<ArteryActor> for Connected {
    fn handle(
        actor: &mut ArteryActor,
        ctx: &mut <ArteryActor as Actor>::Context,
        message: Self,
        _: Option<ActorRef>,
        _: &Receive<ArteryActor>,
    ) -> anyhow::Result<Behavior<ArteryActor>> {
        let Self { stream, peer_addr } = message;
        let framed = FramedWrite::new(stream, PacketCodec);
        let (connection, tx) = OutboundConnection::new(
            peer_addr,
            framed,
            ctx.myself().clone(),
            ctx.system().clone(),
            actor.registry.clone(),
        );
        actor
            .connections
            .insert(peer_addr, ConnectionStatus::Connected(tx));
        info!("{} connected to {}", ctx.myself(), peer_addr);
        if let Some(buffers) = actor.message_buffer.remove(&peer_addr) {
            for envelope in buffers {
                let (message, sender) = envelope.into_inner();
                ctx.myself().tell(message, sender);
            }
        }
        ctx.spawn_async(format!("connection_out_{}", peer_addr), async move {
            connection.receive_loop().await;
        })?;
        Ok(Behavior::same())
    }
}

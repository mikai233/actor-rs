use std::fmt::Debug;
use std::net::SocketAddr;

use actor_core::actor::behavior::Behavior;
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::actor_ref::ActorRef;
use actor_core::message::handler::MessageHandler;
use actor_core::Message;
use tokio_util::codec::FramedRead;

use crate::artery::ArteryActor;

use super::codec::PacketCodec;
use super::inbound_connection::InboundConnection;

#[derive(Debug, Message, derive_more::Display, derive_more::Constructor)]
#[display("AcceptConnection({})", peer_addr)]
pub(super) struct AcceptConnection {
    pub(super) stream: tokio::net::TcpStream,
    pub(super) peer_addr: SocketAddr,
}

impl MessageHandler<ArteryActor> for AcceptConnection {
    fn handle(
        actor: &mut ArteryActor,
        ctx: &mut <ArteryActor as Actor>::Context,
        message: Self,
        _: Option<ActorRef>,
        _: &Receive<ArteryActor>,
    ) -> anyhow::Result<Behavior<ArteryActor>> {
        let Self { stream, peer_addr } = message;
        let framed = FramedRead::new(stream, PacketCodec);
        let artery = ctx.myself().clone();
        let system = ctx.system().clone();
        let registry = actor.registry.clone();
        let connection = InboundConnection::new(peer_addr, framed, artery, system, registry);
        ctx.spawn_async(format!("connection_in_{}", peer_addr), async move {
            connection.receive_loop().await;
        })?;
        Ok(Behavior::same())
    }
}

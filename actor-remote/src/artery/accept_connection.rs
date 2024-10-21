use std::fmt::Debug;
use std::net::SocketAddr;

use actor_core::actor::behavior::Behavior;
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::actor_ref::ActorRef;
use actor_core::message::handler::MessageHandler;
use actor_core::Message;

use crate::artery::ArteryActor;

#[derive(Debug, Message, derive_more::Display, derive_more::Constructor)]
#[display("AcceptConnection({})", peer_addr)]
pub(super) struct AcceptConnection {
    pub(super) stream: tokio::net::TcpStream,
    pub(super) peer_addr: SocketAddr,
}

impl MessageHandler<ArteryActor> for AcceptConnection {
    fn handle(
        _: &mut ArteryActor,
        ctx: &mut <ArteryActor as Actor>::Context,
        message: Self,
        _: Option<ActorRef>,
        _: &Receive<ArteryActor>,
    ) -> anyhow::Result<Behavior<ArteryActor>> {
        let actor = ctx.myself().clone();
        let Self { stream, peer_addr } = message;
        ctx.spawn_async(format!("connection_in_{}", message.peer_addr), async move {
            ArteryActor::accept_inbound_connection(stream, peer_addr, actor).await
        })?;
        Ok(Behavior::same())
    }
}

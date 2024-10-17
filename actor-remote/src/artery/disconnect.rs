use std::net::SocketAddr;

use actor_core::actor::behavior::Behavior;
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::actor_ref::ActorRef;
use actor_core::message::handler::MessageHandler;
use actor_core::Message;
use tracing::info;

use actor_core::actor::context::ActorContext;

use crate::artery::connection_status::ConnectionStatus;
use crate::artery::ArteryActor;

#[derive(Debug, Message, derive_more::Display)]
#[display("Disconnect {{ addr: {addr} }}")]
pub struct Disconnect {
    pub addr: SocketAddr,
}

impl MessageHandler<ArteryActor> for Disconnect {
    fn handle(
        actor: &mut ArteryActor,
        ctx: &mut <ArteryActor as Actor>::Context,
        message: Self,
        sender: Option<ActorRef>,
        _: &Receive<ArteryActor>,
    ) -> anyhow::Result<Behavior<ArteryActor>> {
        let Self { addr } = message;
        if let Some(ConnectionStatus::Connecting(handle)) = actor.connections.remove(&addr) {
            handle.abort();
        }
        actor.message_buffer.remove(&addr);
        info!("{} disconnect from {}", ctx.myself(), addr);
        Ok(Behavior::same())
    }
}

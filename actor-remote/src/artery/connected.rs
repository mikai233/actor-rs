use std::net::SocketAddr;

use actor_core::actor::behavior::Behavior;
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::actor_ref::ActorRef;
use actor_core::message::handler::MessageHandler;
use actor_core::Message;
use tracing::info;

use actor_core::actor::context::ActorContext;
use actor_core::message::message_buffer::BufferEnvelope;

use crate::artery::connection::ConnectionTx;
use crate::artery::connection_status::ConnectionStatus;
use crate::artery::ArteryActor;

#[derive(Debug, Message, derive_more::Display)]
#[display("Connected {{ addr: {addr}, tx: ... }}")]
pub(super) struct Connected {
    pub(super) addr: SocketAddr,
    pub(super) tx: ConnectionTx,
}

impl MessageHandler<ArteryActor> for Connected {
    fn handle(
        actor: &mut ArteryActor,
        ctx: &mut <ArteryActor as Actor>::Context,
        message: Self,
        sender: Option<ActorRef>,
        _: &Receive<ArteryActor>,
    ) -> anyhow::Result<Behavior<ArteryActor>> {
        let Self { addr, tx } = message;
        actor
            .connections
            .insert(addr, ConnectionStatus::Connected(tx));
        info!("{} connected to {}", ctx.myself(), addr);
        if let Some(buffers) = actor.message_buffer.remove(&addr) {
            for envelope in buffers {
                let (message, sender) = envelope.into_inner();
                ctx.myself().tell(message, sender);
            }
        }
        Ok(Behavior::same())
    }
}

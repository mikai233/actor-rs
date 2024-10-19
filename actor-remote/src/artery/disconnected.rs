use std::net::SocketAddr;

use tracing::info;

use actor_core::{
    actor::{behavior::Behavior, receive::Receive, Actor},
    actor_ref::ActorRef,
    message::handler::MessageHandler,
    Message,
};

use crate::artery::ArteryActor;

#[derive(Debug, Message, derive_more::Display)]
#[display("Disconnected {{ addr: {addr} }}")]
pub(super) struct Disconnected {
    pub(super) addr: SocketAddr,
}

impl MessageHandler<ArteryActor> for Disconnected {
    fn handle(
        actor: &mut ArteryActor,
        ctx: &mut <ArteryActor as Actor>::Context,
        message: Self,
        _: Option<ActorRef>,
        _: &Receive<ArteryActor>,
    ) -> anyhow::Result<Behavior<ArteryActor>> {
        let Self { addr } = message;
        actor.message_buffer.remove(&addr);
        info!("{} disconnected from {}", ctx.myself(), addr);
        Ok(Behavior::same())
    }
}

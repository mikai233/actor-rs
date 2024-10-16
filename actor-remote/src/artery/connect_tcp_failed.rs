use std::net::SocketAddr;

use actor_core::{
    actor::{behavior::Behavior, receive::Receive, Actor},
    actor_ref::ActorRef,
    message::handler::MessageHandler,
    Message,
};

use crate::artery::ArteryActor;

#[derive(Debug, Message, derive_more::Display)]
#[display("ConnectFailed({})", addr)]
pub(super) struct ConnectFailed {
    pub(super) addr: SocketAddr,
}

impl MessageHandler<ArteryActor> for ConnectFailed {
    fn handle(
        actor: &mut ArteryActor,
        _ctx: &mut <ArteryActor as Actor>::Context,
        message: Self,
        _sender: Option<ActorRef>,
        _: &Receive<ArteryActor>,
    ) -> anyhow::Result<Behavior<ArteryActor>> {
        actor.connections.remove(&message.addr);
        Ok(Behavior::same())
    }
}

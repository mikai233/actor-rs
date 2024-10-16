use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;

use actor_core::actor::behavior::Behavior;
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::actor_ref::ActorRef;
use actor_core::message::handler::MessageHandler;
use actor_core::Message;

use crate::artery::ArteryActor;

#[derive(Debug, Message, derive_more::Display)]
#[display("SpawnInbound({})", peer_addr)]
pub(super) struct SpawnInbound {
    pub(super) peer_addr: SocketAddr,
    pub(super) fut: Pin<Box<dyn Future<Output = ()> + Send>>,
}

impl MessageHandler<ArteryActor> for SpawnInbound {
    fn handle(
        actor: &mut ArteryActor,
        ctx: &mut <ArteryActor as Actor>::Context,
        message: Self,
        sender: Option<ActorRef>,
        _: &Receive<ArteryActor>,
    ) -> anyhow::Result<Behavior<ArteryActor>> {
        let context = ctx.context_mut();
        context.spawn_fut(format!("connection_in_{}", message.peer_addr), message.fut)?;
        Ok(Behavior::same())
    }
}

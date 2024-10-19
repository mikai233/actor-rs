use std::fmt::Debug;
use std::net::SocketAddr;

use actor_core::actor::behavior::Behavior;
use actor_core::actor::context::ActorContext;
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::actor_ref::ActorRef;
use actor_core::message::handler::MessageHandler;
use actor_core::Message;
use futures::future::BoxFuture;

use crate::artery::ArteryActor;

#[derive(Message, derive_more::Display)]
#[display("SpawnInbound({})", peer_addr)]
pub(super) struct SpawnInbound {
    pub(super) peer_addr: SocketAddr,
    pub(super) fut: BoxFuture<'static, ()>,
}

impl Debug for SpawnInbound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpawnInbound")
            .field("peer_addr", &self.peer_addr)
            .finish_non_exhaustive()
    }
}

impl MessageHandler<ArteryActor> for SpawnInbound {
    fn handle(
        _: &mut ArteryActor,
        ctx: &mut <ArteryActor as Actor>::Context,
        message: Self,
        _: Option<ActorRef>,
        _: &Receive<ArteryActor>,
    ) -> anyhow::Result<Behavior<ArteryActor>> {
        let context = ctx.context_mut();
        context.spawn_fut(format!("connection_in_{}", message.peer_addr), message.fut)?;
        Ok(Behavior::same())
    }
}

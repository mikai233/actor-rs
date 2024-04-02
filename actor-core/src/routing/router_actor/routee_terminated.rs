use async_trait::async_trait;
use bincode::{Decode, Encode};

use actor_derive::MessageCodec;

use crate::actor::context::ActorContext;
use crate::actor_ref::ActorRef;
use crate::Message;
use crate::routing::router_actor::RouterActor;

#[derive(Decode, Encode, MessageCodec)]
pub(crate) struct RouteeTerminated(ActorRef);

#[async_trait]
impl Message for RouteeTerminated {
    type A = RouterActor;

    async fn handle(self: Box<Self>, _context: &mut ActorContext, _actor: &mut Self::A) -> eyre::Result<()> {
        let terminated_routee = self.0;
        todo!()
    }
}
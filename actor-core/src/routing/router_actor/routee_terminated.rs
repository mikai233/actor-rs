use async_trait::async_trait;
use bincode::{Decode, Encode};

use actor_derive::MessageCodec;

use crate::actor::actor_ref::ActorRef;
use crate::actor::context::ActorContext;
use crate::Message;
use crate::message::terminated::Terminated;
use crate::routing::router_actor::RouterActor;

#[derive(Decode, Encode, MessageCodec)]
pub(crate) struct RouteeTerminated(ActorRef);

#[async_trait]
impl Message for RouteeTerminated {
    type A = RouterActor;

    async fn handle(self: Box<Self>, _context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
        todo!()
    }
}

impl Terminated for RouteeTerminated {
    fn actor(&self) -> &ActorRef {
        &self.0
    }
}
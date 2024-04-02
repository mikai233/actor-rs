use async_trait::async_trait;

use actor_derive::EmptyCodec;

use crate::actor::context::ActorContext;
use crate::Message;
use crate::routing::routee::Routee;
use crate::routing::router_actor::Router;

#[derive(EmptyCodec)]
pub struct AddRoutee {
    pub routee: Routee,
}

#[async_trait]
impl Message for AddRoutee {
    type A = Box<dyn Router>;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> eyre::Result<()> {
        let Self { routee, .. } = *self;
        actor.routees_mut().push(routee);
        Ok(())
    }
}
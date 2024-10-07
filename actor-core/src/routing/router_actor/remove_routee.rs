use async_trait::async_trait;

use actor_derive::EmptyCodec;

use crate::actor::context::ActorContext1;
use crate::Message;
use crate::routing::routee::Routee;
use crate::routing::router_actor::Router;

#[derive(EmptyCodec)]
pub struct RemoveRoutee {
    pub routee: Routee,
}

#[async_trait]
impl Message for RemoveRoutee {
    type A = Box<dyn Router>;

    async fn handle(self: Box<Self>, _context: &mut ActorContext1, actor: &mut Self::A) -> anyhow::Result<()> {
        let Self { routee: remove_routee } = *self;
        actor.routees_mut().retain(|routee| *routee != remove_routee);
        Ok(())
    }
}
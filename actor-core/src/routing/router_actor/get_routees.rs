use async_trait::async_trait;

use actor_derive::{EmptyCodec, OrphanEmptyCodec};

use crate::actor::context::ActorContext;
use crate::actor_ref::ActorRefExt;
use crate::ext::option_ext::OptionExt;
use crate::Message;
use crate::routing::routee::Routee;
use crate::routing::router_actor::Router;

#[derive(Debug, EmptyCodec)]
pub struct GetRoutees;

#[async_trait]
impl Message for GetRoutees {
    type A = Box<dyn Router>;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> eyre::Result<()> {
        let routees = actor.routees().clone();
        context.sender.foreach(move |sender| {
            sender.resp(GetRouteesResp { routees });
        });
        Ok(())
    }
}

#[derive(OrphanEmptyCodec)]
pub struct GetRouteesResp {
    pub routees: Vec<Routee>,
}
use async_trait::async_trait;

use actor_derive::{EmptyCodec, OrphanEmptyCodec};

use crate::actor::context::Context;
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

    async fn handle(self: Box<Self>, context: &mut Context, actor: &mut Self::A) -> anyhow::Result<()> {
        let routees = actor.routees().clone();
        context.sender.foreach(move |sender| {
            sender.cast_orphan_ns(GetRouteesResp { routees });
        });
        Ok(())
    }
}

#[derive(OrphanEmptyCodec)]
pub struct GetRouteesResp {
    pub routees: Vec<Routee>,
}
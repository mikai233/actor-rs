use crate::actor::behavior::Behavior;
use crate::actor::receive::Receive;
use crate::actor_ref::{ActorRef, ActorRefExt};
use crate::message::handler::MessageHandler;
use crate::routing::routee::Routee;
use crate::routing::router_actor::Router;
use actor_derive::Message;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[derive(Debug, Serialize, Deserialize, Message, derive_more::Display)]
#[display("GetRoutees")]
pub struct GetRoutees;

impl<A: Router> MessageHandler<A> for GetRoutees {
    fn handle(
        actor: &mut A,
        _: &mut A::Context,
        _: Self,
        sender: Option<ActorRef>,
        _: &Receive<A>,
    ) -> anyhow::Result<Behavior<A>> {
        let routees = actor.routees().clone();
        if let Some(sender) = sender {
            sender.cast_ns(GetRouteesResp { routees });
        }
        Ok(Behavior::same())
    }
}

#[derive(Debug, Message)]
pub struct GetRouteesResp {
    pub routees: Vec<Routee>,
}

impl Display for GetRouteesResp {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "GetRouteesResp{{ routees: {} }}",
            self.routees.iter().join(", ")
        )
    }
}

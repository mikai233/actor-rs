use std::fmt::{Debug, Formatter};

use actor_derive::EmptyCodec;

use crate::context::ActorContext;
use crate::Message;
use crate::props::Props;
use crate::routing::router::{Routee, Router};
use crate::routing::router_actor::RouterActor;
use crate::system::ActorSystem;

pub trait RouterConfig {
    fn create_router(&self, system: ActorSystem) -> Router;
    fn create_router_actor(&self) -> RouterActor;
}

pub trait Pool: RouterConfig {
    fn nr_of_instances(&self, sys: &ActorSystem) -> usize;
    fn new_routee(&self, routee_props: Props, context: ActorContext) -> Box<dyn Routee>;
    fn props(self, routee_props: Props) -> Props {
        routee_props.with_router(self)
    }
}

#[derive(EmptyCodec)]
pub enum RouterMessage {
    GetRoutees,
    AddRoutee(Box<dyn Routee>),
    RemoveRoutee(Box<dyn Routee>),
}

impl Debug for RouterMessage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self {
            RouterMessage::GetRoutees => f.debug_struct("GetRoutees")
                .finish(),
            RouterMessage::AddRoutee(_) => f.debug_struct("AddRoutee")
                .finish_non_exhaustive(),
            RouterMessage::RemoveRoutee(_) => f.debug_struct("RemoveRoutee")
                .finish_non_exhaustive(),
        }
    }
}

impl Message for RouterMessage {
    type A = RouterActor;

    fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        todo!()
    }
}
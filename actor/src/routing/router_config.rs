use std::fmt::{Debug, Formatter};

use dyn_clone::DynClone;

use actor_derive::EmptyCodec;

use crate::{DynMessage, Message, MessageType};
use crate::context::ActorContext;
use crate::props::Props;
use crate::routing::router::{Routee, Router};
use crate::routing::router_actor::{RouterActor, WatchRouteeTerminated};
use crate::system::ActorSystem;

pub trait RouterConfig: DynClone {
    fn create_router(&self, system: ActorSystem) -> Router;
    fn create_router_actor(&self) -> RouterActor;
    fn is_management_message(&self, message: &DynMessage) -> bool {
        if message.name == std::any::type_name::<WatchRouteeTerminated>()
            || matches!(message.message_type, MessageType::System) {
            true
        } else {
            false
        }
    }
}

pub trait Pool: RouterConfig {
    fn nr_of_instances(&self, sys: &ActorSystem) -> usize;
    fn new_routee(&self, routee_props: Props, context: ActorContext) -> Box<dyn Routee>;
    fn props(&self, routee_props: Props) -> Props;
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
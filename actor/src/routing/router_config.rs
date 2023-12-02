use std::fmt::{Debug, Formatter};

use dyn_clone::DynClone;
use enum_dispatch::enum_dispatch;

use actor_derive::EmptyCodec;

use crate::{DynMessage, Message, MessageType};
use crate::context::ActorContext;
use crate::props::Props;
use crate::routing::group_router_config::GroupRouterConfig;
use crate::routing::pool_router_config::PoolRouterConfig;
use crate::routing::router::{Routee, Router};
use crate::routing::router_actor::{RouterActor, WatchRouteeTerminated};
use crate::system::ActorSystem;

pub trait TRouterConfig: Send + Sync + DynClone + 'static {
    fn create_router(&self, system: ActorSystem) -> Router;
    fn create_router_actor(&self) -> RouterActor;
    fn is_management_message(&self, message: &DynMessage) -> bool {
        if matches!(message.message_type, MessageType::System)
            || message.name == std::any::type_name::<WatchRouteeTerminated>()
            || message.name == std::any::type_name::<RouterMessage>() {
            true
        } else {
            false
        }
    }
    fn stop_router_when_all_routees_removed(&self) -> bool {
        true
    }
}

pub trait Pool: TRouterConfig {
    fn nr_of_instances(&self, sys: &ActorSystem) -> usize;
    fn new_routee(&self, routee_props: Props, context: ActorContext) -> Box<dyn Routee>;
    fn props(&self, routee_props: Props) -> Props;
}

dyn_clone::clone_trait_object!(Pool);

pub trait Group: TRouterConfig {
    fn paths(&self, system: &ActorSystem) -> Vec<String>;
    fn props(&self) -> Props;
    fn routee_for(&self, path: &String, context: &mut ActorContext) -> Box<dyn Routee>;
}

dyn_clone::clone_trait_object!(Group);

#[enum_dispatch(TRouterConfig)]
pub enum RouterConfig {
    PoolRouterConfig,
    GroupRouterConfig,
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
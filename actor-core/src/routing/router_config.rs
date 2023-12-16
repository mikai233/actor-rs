use std::fmt::{Debug, Formatter};
use std::sync::Arc;

use dyn_clone::DynClone;
use enum_dispatch::enum_dispatch;

use actor_derive::{EmptyCodec, UntypedMessageEmptyCodec};

use crate::{DynMessage, Message, MessageType};
use crate::actor::actor_ref::ActorRefExt;
use crate::actor::actor_ref_factory::ActorRefFactory;
use crate::actor::actor_system::ActorSystem;
use crate::actor::context::{ActorContext, Context};
use crate::actor::fault_handing::SupervisorStrategy;
use crate::actor::props::Props;
use crate::ext::option_ext::OptionExt;
use crate::routing::group_router_config::GroupRouterConfig;
use crate::routing::pool_router_config::PoolRouterConfig;
use crate::routing::router::{ActorRefRoutee, Routee, Router};
use crate::routing::router_actor::{RouterActor, WatchRouteeTerminated};

#[enum_dispatch(RouterConfig)]
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

    fn new_routee(&self, routee_props: Props, context: &mut ActorContext) -> anyhow::Result<Box<dyn Routee>> {
        let routee = context.spawn_anonymous_actor(routee_props)?;
        let routee = ActorRefRoutee(routee);
        Ok(Box::new(routee))
    }

    fn props(&self, routee_props: Props) -> Props;

    fn supervisor_strategy(&self) -> &Box<dyn SupervisorStrategy>;
}

dyn_clone::clone_trait_object!(Pool);

pub trait Group: TRouterConfig {
    fn paths(&self, system: &ActorSystem) -> Vec<String>;
    fn props(&self) -> Props;
    fn routee_for(&self, path: &String, context: &mut ActorContext) -> Box<dyn Routee>;
}

dyn_clone::clone_trait_object!(Group);

#[enum_dispatch]
#[derive(Clone)]
pub enum RouterConfig {
    PoolRouterConfig,
    GroupRouterConfig,
}

#[derive(EmptyCodec)]
pub enum RouterMessage {
    GetRoutees,
    AddRoutee(Arc<Box<dyn Routee>>),
    RemoveRoutee(Arc<Box<dyn Routee>>),
}

impl Debug for RouterMessage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self {
            RouterMessage::GetRoutees => f.debug_struct("GetRoutees")
                .finish(),
            RouterMessage::AddRoutee(_) => f.debug_tuple("AddRoutee")
                .finish(),
            RouterMessage::RemoveRoutee(_) => f.debug_tuple("RemoveRoutee")
                .finish(),
        }
    }
}

impl Message for RouterMessage {
    type A = RouterActor;

    fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let routed_ref = unsafe { actor.routed_ref.assume_init_ref() };
        match *self {
            RouterMessage::GetRoutees => {
                let routees = routed_ref.router.load().routees.clone();
                let resp = GetRouteesResp { routees };
                context.sender().foreach(|sender| {
                    sender.resp(resp);
                })
            }
            RouterMessage::AddRoutee(routee) => {
                routed_ref.add_routees(vec![routee]);
            }
            RouterMessage::RemoveRoutee(routee) => {
                routed_ref.remove_routee(routee);
            }
        }
        Ok(())
    }
}

#[derive(UntypedMessageEmptyCodec)]
pub struct GetRouteesResp {
    pub routees: Vec<Arc<Box<dyn Routee>>>,
}
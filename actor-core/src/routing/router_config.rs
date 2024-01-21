use std::fmt::{Debug, Formatter};
use std::sync::Arc;

use async_trait::async_trait;
use dyn_clone::DynClone;
use enum_dispatch::enum_dispatch;
use tracing::trace;

use actor_derive::{EmptyCodec, OrphanEmptyCodec};

use crate::{DynMessage, Message, MessageType};
use crate::actor::actor_ref::ActorRefExt;
use crate::actor::actor_ref_factory::ActorRefFactory;
use crate::actor::actor_system::ActorSystem;
use crate::actor::context::ActorContext;
use crate::actor::fault_handing::SupervisorStrategy;
use crate::actor::props::Props;
use crate::ext::option_ext::OptionExt;
use crate::routing::group_router_config::GroupRouterConfig;
use crate::routing::pool_router_config::PoolRouterConfig;
use crate::routing::router::{ActorRefRoutee, Routee, Router};
use crate::routing::router_actor::{TRouterActor, WatchRouteeTerminated};

#[enum_dispatch(RouterConfig)]
pub trait TRouterConfig: Send + Sync + DynClone + 'static {
    fn create_router(&self) -> Router;
    fn create_router_actor(&self, routee_props: Props) -> Box<dyn TRouterActor>;
    fn is_management_message(&self, message: &DynMessage) -> bool {
        if matches!(message.message_type, MessageType::System)
            || message.name == std::any::type_name::<WatchRouteeTerminated>()
            || message.name == std::any::type_name::<GetRoutees>()
            || message.name == std::any::type_name::<AddRoutee>()
            || message.name == std::any::type_name::<RemoveRoutee>() {
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
        let routee = context.spawn_anonymous(routee_props)?;
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

impl Debug for RouterConfig {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            RouterConfig::PoolRouterConfig(_) => {
                f.debug_struct("PoolRouterConfig")
                    .finish()
            }
            RouterConfig::GroupRouterConfig(_) => {
                f.debug_struct("GroupRouterConfig")
                    .finish()
            }
        }
    }
}

#[derive(Debug, EmptyCodec)]
pub struct GetRoutees;

#[async_trait]
impl Message for GetRoutees {
    type A = Box<dyn TRouterActor>;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let routees = actor.router().routees.clone();
        context.sender.foreach(move |sender| {
            sender.resp(GetRouteesResp { routees });
        });
        Ok(())
    }
}

#[derive(OrphanEmptyCodec)]
pub struct GetRouteesResp {
    pub routees: Vec<Arc<Box<dyn Routee>>>,
}

#[derive(EmptyCodec)]
pub struct AddRoutee {
    pub routee: Arc<Box<dyn Routee>>,
}

#[async_trait]
impl Message for AddRoutee {
    type A = Box<dyn TRouterActor>;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let Self { routee } = *self;
        actor.router().routees.push(routee);
        trace!("{} add routee", context.myself);
        Ok(())
    }
}

#[derive(EmptyCodec)]
pub struct RemoveRoutee {
    pub routee: Arc<Box<dyn Routee>>,
}

#[async_trait]
impl Message for RemoveRoutee {
    type A = Box<dyn TRouterActor>;

    async fn handle(self: Box<Self>, _context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let Self { routee: remove_routee } = *self;
        actor.router().routees.retain(|routee| !Arc::ptr_eq(&routee, &remove_routee));
        Ok(())
    }
}
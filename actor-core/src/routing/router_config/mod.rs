use std::fmt::{Debug, Formatter};

use enum_dispatch::enum_dispatch;

use crate::actor::props::Props;
use crate::routing::router_config::group::GroupRouterConfig;
use crate::routing::router_config::pool::PoolRouterConfig;
use crate::routing::routing_logic::RoutingLogic;

pub mod pool;
pub mod group;

#[enum_dispatch(RouterConfig)]
pub trait TRouterConfig: Send {
    fn routing_logic(&self) -> Box<dyn RoutingLogic>;

    fn stop_router_when_all_routees_removed(&self) -> bool {
        true
    }

    fn props(&self) -> Props;
}

#[enum_dispatch]
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

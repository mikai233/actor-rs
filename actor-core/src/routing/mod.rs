use crate::actor::context::Context;
use crate::actor::props::PropsBuilder;
use crate::actor_ref::actor_ref_factory::ActorRefFactory;
use crate::routing::routee::actor_ref_routee::ActorRefRoutee;

pub mod router_config;
pub mod router_actor;
pub mod round_robin_pool;
pub mod routee;
pub mod routing_logic;
mod broadcast_pool;

fn spawn_actor_routee<Arg>(context: &mut Context, builder: &PropsBuilder<Arg>, arg: Arg) -> anyhow::Result<ActorRefRoutee> {
    let routee_props = builder.props(arg);
    let routee = context.spawn_anonymous(routee_props)?;
    Ok(ActorRefRoutee(routee))
}
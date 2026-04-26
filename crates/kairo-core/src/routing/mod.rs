use crate::actor::context::ActorContext;
use crate::actor::props::PropsBuilder;
use crate::actor_ref::actor_ref_factory::ActorRefFactory;
use crate::routing::routee::actor_ref_routee::ActorRefRoutee;

mod broadcast_pool;
pub mod round_robin_pool;
pub mod routee;
pub mod router_actor;
pub mod router_config;
pub mod routing_logic;

fn spawn_actor_routee<Arg>(
    context: &mut ActorContext,
    builder: &PropsBuilder<Arg>,
    arg: Arg,
) -> anyhow::Result<ActorRefRoutee> {
    let routee_props = builder.props(arg);
    let routee = context.spawn_anonymous(routee_props)?;
    Ok(ActorRefRoutee(routee))
}

use std::time::Duration;

use remove_routee::RemoveRoutee;

use crate::actor::behavior::Behavior;
use crate::actor::context::{ActorContext, Context};
use crate::actor::receive::Receive;
use crate::actor::Actor;
use crate::actor_path::TActorPath;
use crate::actor_ref::actor_ref_factory::ActorRefFactory;
use crate::actor_ref::ActorRefExt;
use crate::message::poison_pill::PoisonPill;
use crate::message::terminated::Terminated;
use crate::routing::routee::Routee;
use crate::routing::router_actor::add_routee::AddRoutee;
use crate::routing::router_actor::get_routees::GetRoutees;
use crate::routing::router_config::pool::Pool;
use crate::routing::router_config::RouterConfig;

use super::routee::actor_ref_routee::ActorRefRoutee;
use super::routee::TRoutee;
use super::router_config::TRouterConfig;

pub mod add_routee;
pub mod broadcast;
pub mod get_routees;
pub mod remove_routee;
pub mod routee_envelope;

pub trait Router: Actor {
    fn router_config(&self) -> &RouterConfig;

    fn routees_mut(&mut self) -> &mut Vec<Routee>;

    fn routees(&self) -> &Vec<Routee>;
}

#[derive(Debug)]
pub struct RouterActor {
    routees: Vec<Routee>,
    router_config: RouterConfig,
}

impl RouterActor {
    pub fn new(router_config: RouterConfig) -> Self {
        Self {
            routees: Default::default(),
            router_config,
        }
    }

    fn stop_if_all_routees_removed(&mut self, ctx: &mut <RouterActor as Actor>::Context) {
        if self.routees.is_empty() && self.router_config.stop_router_when_all_routees_removed() {
            ctx.stop(ctx.myself());
        }
    }

    fn remove_routee(&mut self, ctx: &mut <RouterActor as Actor>::Context, routee: Routee) {
        self.routees.retain(|r| *r != routee);
        if let Routee::ActorRefRoutee(routee) = &routee {
            let context = ctx.context_mut();
            context.unwatch(routee);
            if let Some(child_ref) = context.children().get(routee.path().name()) {
                let child_ref = child_ref.clone();
                context
                    .system
                    .scheduler
                    .schedule_once(Duration::from_millis(100), move || {
                        child_ref.cast_ns(PoisonPill);
                    });
            }
        }
        self.stop_if_all_routees_removed(ctx);
    }
}

impl Actor for RouterActor {
    type Context = Context;

    fn started(&mut self, ctx: &mut Self::Context) -> anyhow::Result<()> {
        let context = ctx.context_mut();
        match &self.router_config {
            RouterConfig::PoolRouterConfig(pool) => {
                let n = pool.nr_of_instances(context.system());
                for _ in 0..n {
                    let routee = pool.new_routee(context)?;
                    self.routees.push(routee);
                }
            }
            RouterConfig::GroupRouterConfig(_) => {}
        }
        Ok(())
    }

    fn receive(&self) -> Receive<Self> {
        Receive::new()
            .handle::<GetRoutees>()
            .handle::<AddRoutee>()
            .is::<RemoveRoutee>(|actor: &mut RouterActor, ctx, message, _, _| {
                actor.remove_routee(ctx, message.routee);
                Ok(Behavior::same())
            })
            .is::<Terminated>(|actor: &mut RouterActor, ctx, message, _, _| {
                actor.remove_routee(ctx, ActorRefRoutee(message.actor).into());
                Ok(Behavior::same())
            })
            .any(|actor, _, message, sender, _| {
                let routee = actor
                    .router_config()
                    .routing_logic()
                    .select(&message, actor.routees());
                routee.send(message, sender);
                Ok(Behavior::same())
            })
    }
}

impl Router for RouterActor {
    fn router_config(&self) -> &RouterConfig {
        &self.router_config
    }

    fn routees_mut(&mut self) -> &mut Vec<Routee> {
        &mut self.routees
    }

    fn routees(&self) -> &Vec<Routee> {
        &self.routees
    }
}

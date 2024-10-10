use crate::actor::behavior::Behavior;
use crate::actor::context::ActorContext;
use crate::actor::Actor;
use crate::actor_ref::ActorRef;
use actor_derive::Message;
use serde::{Deserialize, Serialize};
use tracing::{debug, error};

use super::handler::MessageHandler;

#[derive(Debug, Clone, Message, derive_more::Display, Serialize, Deserialize)]
#[cloneable]
#[display("Unwatch {{ watchee: {watchee}, watcher: {watcher} }}")]
pub struct Unwatch {
    pub watchee: ActorRef,
    pub watcher: ActorRef,
}

impl<A: Actor> MessageHandler<A> for Unwatch {
    fn handle(
        actor: &mut A,
        ctx: &mut <A as Actor>::Context,
        message: Self,
        _: Option<ActorRef>,
    ) -> anyhow::Result<Behavior<A>> {
        let Self { watchee, watcher } = message;
        let context = ctx.context_mut();
        let watchee_self = watchee == context.myself;
        let watcher_self = watcher == context.myself;
        if watchee_self && !watcher_self {
            if context.watched_by.contains(&watcher) {
                context.maintain_address_terminated_subscription(Some(&watcher), |ctx| {
                    ctx.watched_by.remove(&watcher);
                    debug!("{} no longer watched by {}", ctx.myself, watcher);
                });
            }
        } else if !watchee_self && watcher_self {
            context.unwatch(&watchee);
        } else {
            error!(
                "illegal Unwatch({},{}) for {}",
                watchee, watcher, context.myself
            );
        }
        Ok(Behavior::same())
    }
}

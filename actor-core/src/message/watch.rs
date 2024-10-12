use crate::{
    actor::{behavior::Behavior, context::ActorContext, receive::Receive, Actor},
    actor_ref::ActorRef,
};
use actor_derive::Message;
use serde::{Deserialize, Serialize};
use tracing::{debug, error};

use super::handler::MessageHandler;

#[derive(Debug, Serialize, Deserialize, Message, derive_more::Display)]
#[display("Watch {{ watchee: {}, watcher: {} }}", watchee, watcher)]
pub struct Watch {
    pub watchee: ActorRef,
    pub watcher: ActorRef,
}

impl<A: Actor> MessageHandler<A> for Watch {
    fn handle(
        _: &mut A,
        ctx: &mut <A as Actor>::Context,
        message: Self,
        _: Option<ActorRef>,
        _: &Receive<A>,
    ) -> anyhow::Result<Behavior<A>> {
        let Watch { watchee, watcher } = message;
        let context = ctx.context_mut();
        let watchee_self = watchee == context.myself;
        let watcher_self = watcher == context.myself;
        if watchee_self && !watcher_self {
            if !context.watched_by.contains(&watcher) {
                context.maintain_address_terminated_subscription(Some(&watcher), |ctx| {
                    debug!("{} is watched by {}", ctx.myself, watcher);
                    ctx.watched_by.insert(watcher.clone());
                });
            } else {
                debug!("watcher {} already added for {}", watcher, context.myself);
            }
        } else {
            error!(
                "illegal Watch({},{}) for {}",
                watchee, watcher, context.myself
            );
        }
        Ok(Behavior::same())
    }
}

use crate::actor::context::{Context, ActorContext};
use crate::actor_ref::ActorRef;
use actor_derive::Message;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, error};

#[derive(Debug, Clone, Message, derive_more::Display, Serialize, Deserialize)]
#[cloneable]
#[display("Unwatch {{ watchee: {watchee}, watcher: {watcher} }}")]
pub struct Unwatch {
    pub watchee: ActorRef,
    pub watcher: ActorRef,
}

#[async_trait]
impl SystemMessage for Unwatch {
    async fn handle(self: Box<Self>, context: &mut Context, _actor: &mut dyn Actor) -> anyhow::Result<()> {
        let Unwatch { watchee, watcher } = *self;
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
            error!("illegal Unwatch({},{}) for {}", watchee, watcher, context.myself);
        }
        Ok(())
    }
}
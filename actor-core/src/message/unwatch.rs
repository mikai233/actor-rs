use async_trait::async_trait;
use bincode::{Decode, Encode};
use tracing::{debug, error};

use actor_derive::SystemCodec;

use crate::{Actor, SystemMessage};
use crate::actor::context::{ActorContext, Context};
use crate::actor_ref::ActorRef;

#[derive(Debug, Encode, Decode, SystemCodec)]
pub struct Unwatch {
    pub(crate) watchee: ActorRef,
    pub(crate) watcher: ActorRef,
}

#[async_trait]
impl SystemMessage for Unwatch {
    async fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut dyn Actor) -> anyhow::Result<()> {
        let Unwatch { watchee, watcher } = *self;
        let watchee_self = watchee == context.myself;
        let watcher_self = watcher == context.myself;
        if watchee_self && !watcher_self {
            if context.watched_by.remove(&watcher) {
                debug!("{} no longer watched by {}", context.myself, watcher);
            }
        } else if !watchee_self && watcher_self {
            context.unwatch(&watchee);
        } else {
            error!("illegal Unwatch({},{}) for {}", watchee, watcher, context.myself);
        }
        Ok(())
    }
}
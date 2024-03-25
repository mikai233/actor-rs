use async_trait::async_trait;
use bincode::{Decode, Encode};
use tracing::{debug, error};

use actor_derive::SystemCodec;

use crate::{Actor, SystemMessage};
use crate::actor::context::ActorContext;
use crate::actor_ref::ActorRef;

#[derive(Debug, Encode, Decode, SystemCodec)]
pub struct Watch {
    pub watchee: ActorRef,
    pub watcher: ActorRef,
}

#[async_trait]
impl SystemMessage for Watch {
    async fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut dyn Actor) -> anyhow::Result<()> {
        let Watch { watchee, watcher } = *self;
        let watchee_self = watchee == context.myself;
        let watcher_self = watcher == context.myself;
        if watchee_self && !watcher_self {
            if !context.watched_by.contains(&watcher) {
                debug!("{} is watched by {}", context.myself, watcher);
                context.watched_by.insert(watcher);
            } else {
                debug!("watcher {} already added for {}", watcher, context.myself);
            }
        } else {
            error!("illegal Watch({},{}) for {}", watchee, watcher, context.myself);
        }
        Ok(())
    }
}
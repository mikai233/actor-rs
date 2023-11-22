use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, error};

use actor_derive::SystemMessageCodec;

use crate::{ SystemMessage};
use crate::actor_ref::SerializedActorRef;
use crate::context::{ActorContext, Context};
use crate::provider::{ActorRefFactory, TActorRefProvider};

#[derive(Debug, Serialize, Deserialize, SystemMessageCodec)]
pub(crate) struct Unwatch {
    pub(crate) watchee: SerializedActorRef,
    pub(crate) watcher: SerializedActorRef,
}

#[async_trait]
impl SystemMessage for Unwatch {
    async fn handle(self: Box<Self>, context: &mut ActorContext) -> anyhow::Result<()> {
        let Unwatch { watchee, watcher } = *self;
        let watchee = context.system.provider().resolve_actor_ref_of_path(&watchee.parse_to_path()?);
        let watcher = context.system.provider().resolve_actor_ref_of_path(&watcher.parse_to_path()?);
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
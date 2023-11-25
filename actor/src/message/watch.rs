use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, error};

use actor_derive::SystemMessageCodec;

use crate::{SystemMessage};
use crate::actor_ref::{SerializedActorRef, TActorRef};
use crate::context::ActorContext;
use crate::provider::{ActorRefFactory, TActorRefProvider};

#[derive(Debug, Serialize, Deserialize, SystemMessageCodec)]
pub(crate) struct Watch {
    pub(crate) watchee: SerializedActorRef,
    pub(crate) watcher: SerializedActorRef,
}

#[async_trait]
impl SystemMessage for Watch {
    async fn handle(self: Box<Self>, context: &mut ActorContext) -> anyhow::Result<()> {
        let Watch { watchee, watcher } = *self;
        let watchee = context.system.provider().resolve_actor_ref_of_path(&watchee.parse_to_path()?);
        let watcher = context.system.provider().resolve_actor_ref_of_path(&watcher.parse_to_path()?);
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
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use actor_derive::SystemMessageCodec;

use crate::actor::SystemMessage;
use crate::actor::context::ActorContext;
use crate::actor_ref::SerializedActorRef;
use crate::provider::{ActorRefFactory, TActorRefProvider};

#[derive(Serialize, Deserialize, SystemMessageCodec)]
pub(crate) struct DeathWatchNotification(pub(crate) SerializedActorRef);

#[async_trait]
impl SystemMessage for DeathWatchNotification {
    async fn handle(self: Box<Self>, context: &mut ActorContext) -> anyhow::Result<()> {
        let actor_ref = context.system().provider().resolve_actor_ref(&self.0.path);
        context.watched_actor_terminated(actor_ref);
        Ok(())
    }
}
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use actor_derive::SystemMessageCodec;

use crate::actor_ref::SerializedActorRef;
use crate::context::ActorContext;
use crate::provider::{ActorRefFactory, TActorRefProvider};
use crate::SystemMessage;

#[derive(Serialize, Deserialize, SystemMessageCodec)]
pub(crate) struct DeathWatchNotification(pub(crate) SerializedActorRef);

#[async_trait]
impl SystemMessage for DeathWatchNotification {
    async fn handle(self: Box<Self>, context: &mut ActorContext) -> anyhow::Result<()> {
        //TODO 当停止的Actor是本地Actor时，此时用resolve_actor_ref是拿不到ActorRef的引用的，只能拿到DeadLetterRef，这个时候就不正确了
        let actor_ref = context.system().provider().resolve_actor_ref(&self.0.path);
        context.watched_actor_terminated(actor_ref);
        Ok(())
    }
}
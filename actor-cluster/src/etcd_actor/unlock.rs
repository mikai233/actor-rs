use async_trait::async_trait;
use etcd_client::UnlockResponse;

use actor_core::{DynMessage, Message};
use actor_core::actor::context::ActorContext;
use actor_core::actor_ref::ActorRef;
use actor_derive::{EmptyCodec, OrphanEmptyCodec};

use crate::etcd_actor::EtcdActor;

#[derive(Debug, EmptyCodec)]
pub struct Unlock {
    pub key: Vec<u8>,
    pub applicant: ActorRef,
}

#[async_trait]
impl Message for Unlock {
    type A = EtcdActor;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let mut client = actor.client.clone();
        context.spawn_fut(async move {
            match client.unlock(self.key).await {
                Ok(resp) => {
                    let success = UnlockResult::Success(resp);
                    self.applicant.tell(DynMessage::orphan(success), ActorRef::no_sender());
                }
                Err(error) => {
                    let failed = UnlockResult::Failed(error);
                    self.applicant.tell(DynMessage::orphan(failed), ActorRef::no_sender());
                }
            }
        });
        Ok(())
    }
}

#[derive(Debug, OrphanEmptyCodec)]
pub enum UnlockResult {
    Success(UnlockResponse),
    Failed(etcd_client::Error),
}

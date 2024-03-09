use std::time::Duration;

use async_trait::async_trait;
use etcd_client::{LockOptions, LockResponse};

use actor_core::{DynMessage, Message};
use actor_core::actor::context::ActorContext;
use actor_core::actor_ref::ActorRef;
use actor_derive::{EmptyCodec, OrphanEmptyCodec};

use crate::etcd_actor::EtcdActor;

#[derive(Debug, EmptyCodec)]
pub struct Lock {
    pub name: Vec<u8>,
    pub options: Option<LockOptions>,
    pub timeout: Option<Duration>,
    pub applicant: ActorRef,
}

#[async_trait]
impl Message for Lock {
    type A = EtcdActor;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let mut client = actor.client.clone();
        context.spawn_fut(async move {
            match self.timeout {
                None => {
                    match client.lock(self.name, self.options).await {
                        Ok(resp) => {
                            let success = LockResult::Success(resp);
                            self.applicant.tell(DynMessage::orphan(success), ActorRef::no_sender());
                        }
                        Err(error) => {
                            let failed = LockResult::Failed(Some(error));
                            self.applicant.tell(DynMessage::orphan(failed), ActorRef::no_sender());
                        }
                    }
                }
                Some(timeout) => {
                    match tokio::time::timeout(timeout, client.lock(self.name, self.options)).await {
                        Ok(resp) => {
                            match resp {
                                Ok(resp) => {
                                    let success = LockResult::Success(resp);
                                    self.applicant.tell(DynMessage::orphan(success), ActorRef::no_sender());
                                }
                                Err(error) => {
                                    let failed = LockResult::Failed(Some(error));
                                    self.applicant.tell(DynMessage::orphan(failed), ActorRef::no_sender());
                                }
                            }
                        }
                        Err(_) => {
                            let failed = LockResult::Failed(None);
                            self.applicant.tell(DynMessage::orphan(failed), ActorRef::no_sender());
                        }
                    }
                }
            }
        });
        Ok(())
    }
}

#[derive(Debug, OrphanEmptyCodec)]
pub enum LockResult {
    Success(LockResponse),
    Failed(Option<etcd_client::Error>),
}

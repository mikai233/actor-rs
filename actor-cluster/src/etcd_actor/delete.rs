use async_trait::async_trait;
use etcd_client::{DeleteOptions, DeleteResponse};

use actor_core::{DynMessage, Message};
use actor_core::actor::context::ActorContext;
use actor_core::actor_ref::ActorRef;
use actor_core::ext::option_ext::OptionExt;
use actor_derive::{EmptyCodec, OrphanEmptyCodec};

use crate::etcd_actor::etcd_cmd_resp::EtcdCmdResp;
use crate::etcd_actor::EtcdActor;

#[derive(Debug, EmptyCodec)]
pub struct Delete {
    pub key: Vec<u8>,
    pub options: Option<DeleteOptions>,
    pub applicant: Option<ActorRef>,
}

#[async_trait]
impl Message for Delete {
    type A = EtcdActor;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let mut client = actor.client.clone();
        context.spawn_fut(async move {
            match client.delete(self.key, self.options).await {
                Ok(resp) => {
                    self.applicant.foreach(|applicant| {
                        applicant.tell(
                            DynMessage::orphan(EtcdCmdResp::DeleteResp(DeleteResp::Success(resp))),
                            ActorRef::no_sender(),
                        );
                    });
                }
                Err(error) => {
                    self.applicant.foreach(|applicant| {
                        applicant.tell(
                            DynMessage::orphan(EtcdCmdResp::DeleteResp(DeleteResp::Failed(error))),
                            ActorRef::no_sender(),
                        );
                    });
                }
            }
        });
        Ok(())
    }
}

#[derive(Debug, OrphanEmptyCodec)]
pub enum DeleteResp {
    Success(DeleteResponse),
    Failed(etcd_client::Error),
}
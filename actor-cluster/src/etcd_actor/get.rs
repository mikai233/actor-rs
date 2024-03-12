use async_trait::async_trait;
use etcd_client::{GetOptions, GetResponse};

use actor_core::actor::context::ActorContext;
use actor_core::actor_ref::ActorRef;
use actor_core::ext::message_ext::OrphanMessageExt;
use actor_core::Message;
use actor_derive::{EmptyCodec, OrphanEmptyCodec};

use crate::etcd_actor::etcd_cmd_resp::EtcdCmdResp;
use crate::etcd_actor::EtcdActor;

#[derive(Debug, EmptyCodec)]
pub struct Get {
    pub key: Vec<u8>,
    pub options: Option<GetOptions>,
    pub applicant: ActorRef,
}

#[async_trait]
impl Message for Get {
    type A = EtcdActor;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let mut client = actor.client.clone();
        context.spawn_fut(async move {
            match client.get(self.key, self.options).await {
                Ok(resp) => {
                    self.applicant.tell(
                        EtcdCmdResp::GetResp(GetResp::Success(resp)).into_dyn(),
                        ActorRef::no_sender(),
                    );
                }
                Err(error) => {
                    self.applicant.tell(
                        EtcdCmdResp::GetResp(GetResp::Failed(error)).into_dyn(),
                        ActorRef::no_sender(),
                    );
                }
            }
        });
        Ok(())
    }
}

#[derive(Debug, OrphanEmptyCodec)]
pub enum GetResp {
    Success(GetResponse),
    Failed(etcd_client::Error),
}
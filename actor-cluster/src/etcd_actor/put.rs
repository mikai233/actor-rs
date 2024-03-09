use async_trait::async_trait;
use etcd_client::{PutOptions, PutResponse};

use actor_core::{DynMessage, Message};
use actor_core::actor::context::ActorContext;
use actor_core::actor_ref::ActorRef;
use actor_core::ext::option_ext::OptionExt;
use actor_derive::{EmptyCodec, OrphanEmptyCodec};

use crate::etcd_actor::EtcdActor;

#[derive(Debug, EmptyCodec)]
pub struct Put {
    pub key: String,
    pub value: Vec<u8>,
    pub options: Option<PutOptions>,
    pub applicant: Option<ActorRef>,
}

#[async_trait]
impl Message for Put {
    type A = EtcdActor;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let mut client = actor.client.clone();
        context.spawn_fut(async move {
            match client.put(self.key, self.value, self.options).await {
                Ok(resp) => {
                    let resp = PutResp::Success(resp);
                    self.applicant.foreach(|applicant| {
                        applicant.tell(DynMessage::orphan(resp), ActorRef::no_sender());
                    });
                }
                Err(error) => {
                    let resp = PutResp::Failed(error);
                    self.applicant.foreach(|applicant| {
                        applicant.tell(DynMessage::orphan(resp), ActorRef::no_sender());
                    });
                }
            }
        });
        Ok(())
    }
}

#[derive(Debug, OrphanEmptyCodec)]
pub enum PutResp {
    Success(PutResponse),
    Failed(etcd_client::Error),
}
use async_trait::async_trait;
use etcd_client::PutOptions;

use actor_core::actor::context::ActorContext;
use actor_core::Message;
use actor_derive::{EmptyCodec, OrphanEmptyCodec};

use crate::etcd_actor::EtcdActor;

#[derive(Debug, EmptyCodec)]
pub struct Put {
    pub key: String,
    pub value: Vec<u8>,
    pub options: Option<PutOptions>,
}

#[async_trait]
impl Message for Put {
    type A = EtcdActor;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        todo!()
    }
}

#[derive(Debug, OrphanEmptyCodec)]
pub struct PutFailed {}
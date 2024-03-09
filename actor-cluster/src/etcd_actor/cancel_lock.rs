use async_trait::async_trait;

use actor_core::actor::context::ActorContext;
use actor_core::actor_ref::ActorRef;
use actor_core::Message;
use actor_derive::EmptyCodec;

use crate::etcd_actor::EtcdActor;

#[derive(Debug, EmptyCodec)]
pub struct CancelLock {
    pub key: Vec<u8>,
    pub applicant: ActorRef,
}

#[async_trait]
impl Message for CancelLock {
    type A = EtcdActor;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        todo!()
    }
}
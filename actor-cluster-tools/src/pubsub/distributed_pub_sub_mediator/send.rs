use std::any::type_name;

use anyhow::ensure;
use async_trait::async_trait;

use actor_core::{CodecMessage, DynMessage, EmptyCodec, Message};
use actor_core::actor::context::ActorContext1;

use crate::pubsub::distributed_pub_sub_mediator::DistributedPubSubMediator;

#[derive(Debug, EmptyCodec)]
pub struct Send {
    pub path: String,
    pub msg: DynMessage,
    pub local_affinity: bool,
}

#[async_trait]
impl Message for Send {
    type A = DistributedPubSubMediator;

    async fn handle(self: Box<Self>, context: &mut ActorContext1, actor: &mut Self::A) -> anyhow::Result<()> {
        todo!()
    }
}

impl Send {
    pub fn new<M>(path: String, msg: M) -> anyhow::Result<Self> where M: CodecMessage + Clone {
        ensure!(M::decoder().is_some(), "{} must have a decoder", type_name::<M>());
        let send = Self {
            path,
            msg: msg.into_dyn(),
            local_affinity: false,
        };
        Ok(send)
    }

    pub fn new_dynamic(path: String, msg: DynMessage) -> Self {
        Self {
            path,
            msg,
            local_affinity: false,
        }
    }
}
use std::any::type_name;

use anyhow::ensure;
use async_trait::async_trait;

use actor_core::{CodecMessage, DynMessage, EmptyCodec, Message};
use actor_core::actor::context::ActorContext;

use crate::pubsub::distributed_pub_sub_mediator::DistributedPubSubMediator;

#[derive(Debug, EmptyCodec)]
pub struct SendToAll {
    pub path: String,
    msg: DynMessage,
    pub all_but_self: bool,
}

#[async_trait]
impl Message for SendToAll {
    type A = DistributedPubSubMediator;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        todo!()
    }
}

impl SendToAll {
    pub fn new<M>(path: String, msg: M) -> anyhow::Result<Self> where M: CodecMessage + Clone {
        ensure!(M::decoder().is_some(), "{} must have a decoder", type_name::<M>());
        let send_to_all = Self {
            path,
            msg: msg.into_dyn(),
            all_but_self: false,
        };
        Ok(send_to_all)
    }

    pub fn new_dynamic(path: String, msg: DynMessage) -> anyhow::Result<Self> {
        ensure!(msg.cloneable(), "message must be cloneable");
        let send_to_all = Self {
            path,
            msg,
            all_but_self: false,
        };
        Ok(send_to_all)
    }
}
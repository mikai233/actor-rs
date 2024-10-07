use std::any::type_name;

use anyhow::ensure;
use async_trait::async_trait;

use actor_core::{CodecMessage, DynMessage, EmptyCodec, Message};
use actor_core::actor::context::ActorContext1;

use crate::pubsub::distributed_pub_sub_mediator::DistributedPubSubMediator;

#[derive(Debug, EmptyCodec)]
pub struct Publish {
    topic: String,
    msg: DynMessage,
    pub send_one_message_to_each_group: bool,
}

#[async_trait]
impl Message for Publish {
    type A = DistributedPubSubMediator;

    async fn handle(self: Box<Self>, context: &mut ActorContext1, actor: &mut Self::A) -> anyhow::Result<()> {
        todo!()
    }
}

impl Publish {
    pub fn new<M>(topic: String, msg: M) -> anyhow::Result<Self> where M: CodecMessage + Clone {
        ensure!(topic.len() > 0, "topic must not be empty");
        ensure!(M::decoder().is_some(), "{} must have a decoder", type_name::<M>());
        let publish = Self {
            topic,
            msg: msg.into_dyn(),
            send_one_message_to_each_group: false,
        };
        Ok(publish)
    }

    pub fn new_dynamic(topic: String, msg: DynMessage) -> anyhow::Result<Self> {
        ensure!(topic.len() > 0, "topic must not be empty");
        ensure!(msg.cloneable(), "message must be cloneable");
        let publish = Self {
            topic,
            msg,
            send_one_message_to_each_group: false,
        };
        Ok(publish)
    }

    pub fn topic(&self) -> &str {
        &self.topic
    }

    pub fn msg(&self) -> &DynMessage {
        &self.msg
    }
}
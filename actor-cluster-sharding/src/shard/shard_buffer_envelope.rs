use actor_core::actor_ref::ActorRef;
use actor_core::message::message_buffer::BufferEnvelope;

use crate::message_extractor::ShardEnvelope;
use crate::shard::Shard;

#[derive(Debug)]
pub(super) struct ShardBufferEnvelope {
    pub(super) message: ShardEnvelope<Shard>,
    pub(super) sender: Option<ActorRef>,
}

impl BufferEnvelope for ShardBufferEnvelope {
    type M = ShardEnvelope<Shard>;

    fn message(&self) -> &Self::M {
        &self.message
    }

    fn sender(&self) -> &Option<ActorRef> {
        &self.sender
    }

    fn into_inner(self) -> (Self::M, Option<ActorRef>) {
        let Self { message, sender } = self;
        (message, sender)
    }
}
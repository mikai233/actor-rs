use actor_core::actor_ref::ActorRef;
use actor_core::message::message_buffer::BufferEnvelope;

use crate::message_extractor::ShardEntityEnvelope;

#[derive(Debug)]
pub(super) struct ShardRegionBufferEnvelope {
    pub(super) message: ShardEntityEnvelope,
    pub(super) sender: Option<ActorRef>,
}

impl BufferEnvelope for ShardRegionBufferEnvelope {
    type M = ShardEntityEnvelope;

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
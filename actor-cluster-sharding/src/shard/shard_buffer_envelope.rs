use actor_core::actor::actor_ref::ActorRef;
use actor_core::message::message_buffer::BufferEnvelope;
use crate::shard::shard_envelope::ShardEnvelope;

#[derive(Debug)]
pub(super) struct ShardBufferEnvelope {
    pub(super) envelope: ShardEnvelope,
    pub(super) sender: Option<ActorRef>,
}

impl BufferEnvelope for ShardBufferEnvelope {
    type M = ShardEnvelope;

    fn message(&self) -> &Self::M {
        &self.envelope
    }

    fn sender(&self) -> &Option<ActorRef> {
        &self.sender
    }

    fn into_inner(self) -> (Self::M, Option<ActorRef>) {
        let Self { envelope, sender } = self;
        (envelope, sender)
    }
}
use actor_core::actor_ref::ActorRef;
use actor_core::DynMessage;
use actor_core::message::message_buffer::BufferEnvelope;

#[derive(Debug)]
pub(super) struct ArteryBufferEnvelope {
    pub(super) message: DynMessage,
    pub(super) sender: Option<ActorRef>,
}

impl BufferEnvelope for ArteryBufferEnvelope {
    type M = DynMessage;

    fn message(&self) -> &Self::M {
        &self.message
    }

    fn sender(&self) -> &Option<ActorRef> {
        &self.sender
    }

    fn into_inner(self) -> (Self::M, Option<ActorRef>) {
        let Self { message: envelope, sender } = self;
        (envelope, sender)
    }
}
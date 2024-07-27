use async_trait::async_trait;

use actor_core::actor::context::ActorContext;
use actor_core::actor_ref::PROVIDER;
use actor_core::EmptyCodec;
use actor_core::Message;

use crate::artery::ArteryActor;
use crate::artery::remote_packet::RemotePacket;

#[derive(Debug, EmptyCodec)]
pub(super) struct InboundMessage {
    pub(super) packet: RemotePacket,
}

#[async_trait]
impl Message for InboundMessage {
    type A = ArteryActor;

    async fn handle(self: Box<Self>, _context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let RemotePacket {
            packet,
            sender,
            target,
        } = self.packet;
        let sender = sender.map(|s| actor.resolve_actor_ref(s));
        let target = actor.resolve_actor_ref(target);
        let reg = &actor.registration;
        let message = PROVIDER.sync_scope(actor.provider.clone(), || {
            reg.decode(packet)
        });
        target.tell(message?, sender);
        Ok(())
    }
}
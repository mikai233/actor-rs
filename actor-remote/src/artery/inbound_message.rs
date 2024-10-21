use actor_core::actor::behavior::Behavior;
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::actor_ref::{ActorRef, PROVIDER};
use actor_core::message::handler::MessageHandler;
use actor_core::Message;

use crate::artery::ArteryActor;
use crate::artery::message_packet::MessagePacket;

#[derive(Debug, Message, derive_more::Display)]
#[display("InboundMessage({_0})")]
pub(super) struct InboundMessage(pub(super) MessagePacket);

impl MessageHandler<ArteryActor> for InboundMessage {
    fn handle(
        actor: &mut ArteryActor,
        ctx: &mut <ArteryActor as Actor>::Context,
        message: Self,
        sender: Option<ActorRef>,
        _: &Receive<ArteryActor>,
    ) -> anyhow::Result<Behavior<ArteryActor>> {
        let MessagePacket {
            msg: packet,
            sender,
            target,
        } = message.0;
        let sender = sender.map(|s| actor.resolve_actor_ref(s));
        let target = actor.resolve_actor_ref(target);
        let reg = &actor.registry;
        let message = PROVIDER.sync_scope(actor.provider.clone(), || reg.decode(packet));
        target.tell(message?, sender);
        Ok(Behavior::same())
    }
}

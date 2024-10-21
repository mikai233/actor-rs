use actor_core::actor::behavior::Behavior;
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::actor_ref::ActorRef;
use actor_core::message::handler::MessageHandler;
use actor_core::message::DynMessage;
use actor_core::Message;

use crate::artery::ArteryActor;

#[derive(Debug, Message, derive_more::Display, derive_more::Constructor)]
#[display("InboundMessage {{ message: {message}, sender: {sender:?}, target: {target} }}")]
pub(super) struct InboundMessage {
    pub(super) message: DynMessage,
    pub(super) sender: Option<String>,
    pub(super) target: String,
}

impl MessageHandler<ArteryActor> for InboundMessage {
    fn handle(
        actor: &mut ArteryActor,
        ctx: &mut <ArteryActor as Actor>::Context,
        message: Self,
        _: Option<ActorRef>,
        _: &Receive<ArteryActor>,
    ) -> anyhow::Result<Behavior<ArteryActor>> {
        let Self {
            message,
            sender,
            target,
        } = message;
        let provider = ctx.system().provider();
        let sender = sender.map(|s| actor.resolve_actor_ref(provider, s));
        let target = actor.resolve_actor_ref(provider, target);
        target.tell(message, sender);
        Ok(Behavior::same())
    }
}

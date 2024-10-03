use crate::actor::context::ActorContext;
use crate::actor_ref::ActorRef;
use crate::message::DynMessage;
use crate::Actor;

pub trait MessageHandler {
    type A: Actor;
    fn handle(
        &self,
        actor: &mut Self::A,
        ctx: &mut ActorContext,
        message: DynMessage,
        sender: Option<ActorRef>,
    ) -> anyhow::Result<()>;
}
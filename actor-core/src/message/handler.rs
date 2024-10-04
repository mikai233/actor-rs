use crate::actor::behavior::Behavior;
use crate::actor::{Actor, ActorContext, ActorRef};
use crate::message::Message;

pub trait MessageHandler<A: Actor>: Message + Sized {
    fn handle(
        actor: &mut A,
        ctx: &mut ActorContext<A>,
        message: Self,
        sender: Option<ActorRef>,
    ) -> anyhow::Result<Behavior<A>>;
}

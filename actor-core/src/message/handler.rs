use crate::actor::behavior::Behavior;
use crate::actor::{Actor, ActorRef};
use crate::message::Message;

pub trait MessageHandler<A: Actor>: Message + Sized {
    fn handle(
        actor: &mut A,
        ctx: &mut A::Context,
        message: Self,
        sender: Option<ActorRef>,
    ) -> anyhow::Result<Behavior<A>>;
}

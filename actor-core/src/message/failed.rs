use crate::{
    actor::{
        behavior::Behavior, context::ActorContext, directive::Directive, receive::Receive, Actor,
    },
    actor_ref::ActorRef,
};
use actor_derive::Message;

use super::handler::MessageHandler;

#[derive(Debug, Message, derive_more::Display)]
#[display("Failed {{ child: {}, error: {:?} }}", child, error)]
pub(crate) struct Failed {
    pub(crate) child: ActorRef,
    pub(crate) error: anyhow::Error,
}

impl<A: Actor> MessageHandler<A> for Failed {
    fn handle(
        actor: &mut A,
        ctx: &mut <A as Actor>::Context,
        message: Self,
        _: Option<ActorRef>,
        _: &Receive<A>,
    ) -> anyhow::Result<Behavior<A>> {
        let Failed { child, error } = message;
        let directive = actor.on_child_failure(ctx, &child, &error);
        match directive {
            Directive::Resume => {
                child.resume();
            }
            Directive::Stop => {
                let children = ctx.context().children();
                debug_assert!(children.iter().find(|c| c.value() == &child));
                ctx.stop(&child);
            }
            Directive::Escalate => {
                if let Some(parent) = ctx.context().parent() {
                    parent.cast_ns(Failed { child, error });
                }
            }
        }
        Ok(Behavior::same())
    }
}

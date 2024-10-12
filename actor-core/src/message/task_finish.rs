use crate::{
    actor::{behavior::Behavior, context::ActorContext, receive::Receive, Actor},
    actor_ref::ActorRef,
};
use actor_derive::Message;
use tracing::{error, trace};

use super::handler::MessageHandler;

#[derive(Debug, Message, derive_more::Display)]
#[display("TaskFinish {{ name: {name} }}")]
pub(crate) struct TaskFinish {
    pub(crate) name: String,
}

impl<A: Actor> MessageHandler<A> for TaskFinish {
    fn handle(
        _: &mut A,
        ctx: &mut <A as Actor>::Context,
        message: Self,
        _: Option<ActorRef>,
        _: &Receive<A>,
    ) -> anyhow::Result<Behavior<A>> {
        let context = ctx.context_mut();
        match context.abort_handles.remove(&message.name) {
            None => {
                error!("finish task not found: {}", message.name);
            }
            Some(_) => {
                trace!("finish task: {}", message.name);
            }
        }
        Ok(Behavior::same())
    }
}

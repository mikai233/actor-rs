use actor_derive::Message;
use serde::{Deserialize, Serialize};

use crate::{
    actor::{behavior::Behavior, receive::Receive, Actor},
    actor_ref::ActorRef,
};

use super::handler::MessageHandler;

#[derive(Debug, Copy, Clone, Message, Serialize, Deserialize, derive_more::Display)]
#[cloneable]
#[display("Kill")]
pub struct Kill;

impl<A: Actor> MessageHandler<A> for Kill {
    fn handle(
        _: &mut A,
        _: &mut <A as Actor>::Context,
        _: Self,
        _: Option<ActorRef>,
        _: &Receive<A>,
    ) -> anyhow::Result<Behavior<A>> {
        Err(anyhow::anyhow!("Actor killed"))
    }
}

use actor_derive::Message;

use crate::actor::address::Address;
use crate::actor::behavior::Behavior;
use crate::actor::context::ActorContext;
use crate::actor::receive::Receive;
use crate::actor::Actor;
use crate::actor_path::TActorPath;
use crate::actor_ref::{ActorRef, ActorRefExt};
use crate::message::death_watch_notification::DeathWatchNotification;

use super::handler::MessageHandler;

#[derive(Debug, Clone, Message, derive_more::Display)]
#[cloneable]
#[display("AddressTerminated {{ address: {address} }}")]
pub struct AddressTerminated {
    pub address: Address,
}

impl<A: Actor> MessageHandler<A> for AddressTerminated {
    fn handle(
        _: &mut A,
        ctx: &mut <A as Actor>::Context,
        message: Self,
        _: Option<ActorRef>,
        _: &Receive<A>,
    ) -> anyhow::Result<Behavior<A>> {
        ctx.context_mut()
            .maintain_address_terminated_subscription(None, |ctx| {
                ctx.watched_by
                    .retain(|w| &message.address != w.path().address());
            });
        ctx.context()
            .watching
            .iter()
            .filter(|(w, _)| w.path().address() == &message.address)
            .for_each(|(w, _)| {
                let notification = DeathWatchNotification {
                    actor: w.clone(),
                    existence_confirmed: ctx.context().children().contains_key(w.path().name()),
                    address_terminated: true,
                };
                ctx.context().myself.cast_ns(notification);
            });
        Ok(Behavior::same())
    }
}

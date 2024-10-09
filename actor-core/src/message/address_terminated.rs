use async_trait::async_trait;

use actor_derive::Message;

use crate::actor::address::Address;
use crate::actor::context::{Context, ActorContext};
use crate::actor_path::TActorPath;
use crate::actor_ref::ActorRef;
use crate::message::death_watch_notification::DeathWatchNotification;

#[derive(Debug, Clone, Message, derive_more::Display)]
#[cloneable]
#[display("AddressTerminated {{ address: {address} }}")]
pub struct AddressTerminated {
    pub address: Address,
}

#[async_trait]
impl SystemMessage for AddressTerminated {
    async fn handle(self: Box<Self>, context: &mut Context, _actor: &mut dyn Actor) -> anyhow::Result<()> {
        context.maintain_address_terminated_subscription(None, |ctx| {
            ctx.watched_by.retain(|w| { &self.address != w.path().address() });
        });
        context.watching.iter()
            .filter(|(a, _)| { a.path().address() == &self.address })
            .for_each(|(a, _)| {
                let notification = DeathWatchNotification {
                    actor: a.clone(),
                    existence_confirmed: context.child(a.path().name()).is_some(),
                    address_terminated: true,
                };
                context.myself().cast_system(notification, ActorRef::no_sender());
            });
        Ok(())
    }
}
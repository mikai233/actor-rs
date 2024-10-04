use async_trait::async_trait;
use bincode::{Decode, Encode};

use actor_derive::CSystemCodec;

use crate::actor::address::Address;
use crate::actor::context::{ActorContext, Context};
use crate::actor_path::TActorPath;
use crate::actor_ref::ActorRef;
use crate::message::death_watch_notification::DeathWatchNotification;
use crate::message::handler::MessageHandler;

#[derive(Debug, Clone, Encode, Decode, CSystemCodec)]
pub struct AddressTerminated {
    pub address: Address,
}

impl MessageHandler for AddressTerminated {
    type A = ();

    fn handle(actor: &mut Self::A, ctx: &mut ActorContext<Self::A>, message: Self, sender: Option<ActorRef>) -> anyhow::Result<()> {
        todo!()
    }
}

#[async_trait]
impl SystemMessage for AddressTerminated {
    async fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut dyn Actor) -> anyhow::Result<()> {
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
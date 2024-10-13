use crate::actor::behavior::Behavior;
use crate::actor::{Actor, ActorRef};
use crate::message::handler::MessageHandler;
use crate::message::{downcast_into, DynMessage, Message};
use ahash::{HashMap, HashMapExt};
use std::any::TypeId;

pub type ReceiveFn<A> = Box<
    dyn Fn(
            &mut A,
            &mut <A as Actor>::Context,
            DynMessage,
            Option<ActorRef>,
            &Receive<A>,
        ) -> anyhow::Result<Behavior<A>>
        + Send
        + Sync,
>;

pub struct Receive<A: Actor> {
    pub receiver: HashMap<TypeId, ReceiveFn<A>>,
}

impl<A: Actor> Receive<A> {
    pub fn new() -> Self {
        Self {
            receiver: HashMap::new(),
        }
    }

    pub fn receive(
        &self,
        actor: &mut A,
        ctx: &mut A::Context,
        message: DynMessage,
        sender: Option<ActorRef>,
    ) -> anyhow::Result<Behavior<A>> {
        let signature = message.signature();
        if let Some(receiver) = self.receiver.get(&signature.type_id) {
            receiver(actor, ctx, message, sender, self)
        } else {
            Err(anyhow::anyhow!(
                "No receiver found for message: {}",
                signature
            ))
        }
    }

    pub fn is<M>(
        mut self,
        handler: impl Fn(
                &mut A,
                &mut A::Context,
                M,
                Option<ActorRef>,
                &Receive<A>,
            ) -> anyhow::Result<Behavior<A>>
            + Send
            + Sync
            + 'static,
    ) -> Self
    where
        M: Message,
    {
        let signature = M::signature_sized();
        self.receiver.insert(
            signature.type_id,
            Box::new(move |actor, ctx, message, sender, receive| {
                let message = downcast_into::<M>(message)
                    .map_err(|_| anyhow::anyhow!("Downcast {signature} failed"))?;
                handler(actor, ctx, *message, sender, receive)
            }),
        );
        self
    }

    pub fn handle<M>(self) -> Self
    where
        M: Message + MessageHandler<A>,
    {
        self.is(|actor, ctx, message, sender, receive| {
            M::handle(actor, ctx, message, sender, receive)
        })
    }
}

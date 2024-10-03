use crate::actor::context::ActorContext;
use crate::actor::Actor;
use crate::actor_ref::ActorRef;
use crate::message::{downcast_into, DynMessage, Message};
use ahash::{HashMap, HashMapExt};

pub type ReceiveFn<A> = Box<dyn Fn(&mut A, &mut ActorContext<A>, DynMessage, Option<ActorRef>) -> anyhow::Result<()>>;

pub struct Receive<A: Actor> {
    pub receiver: HashMap<&'static str, ReceiveFn<A>>,
}

impl<A: Actor> Receive<A> {
    pub(crate) fn new() -> Self {
        Self {
            receiver: HashMap::new(),
        }
    }

    pub(crate) fn receive(
        &self,
        actor: &mut A,
        ctx: &mut ActorContext<A>,
        message: Box<dyn Message>,
        sender: Option<ActorRef>,
    ) -> anyhow::Result<()> {
        let signature = message.signature();
        if let Some(receiver) = self.receiver.get(signature) {
            receiver(actor, ctx, message, sender)
        } else {
            Err(anyhow::anyhow!(
                "No receiver found for message: {}",
                signature
            ))
        }
    }

    pub(crate) fn is<M>(
        mut self,
        handler: impl Fn(&mut A, &mut ActorContext<A>, M, Option<ActorRef>) -> anyhow::Result<()> + 'static,
    ) -> Self
    where
        M: Message,
    {
        let signature = M::signature_sized();
        self.receiver.insert(
            signature,
            Box::new(move |actor, ctx, message, sender| {
                let message = downcast_into::<M>(message)
                    .map_err(|_| anyhow::anyhow!("Downcast {signature} failed"))?;
                handler(actor, ctx, *message, sender)
            }),
        );
        self
    }
}

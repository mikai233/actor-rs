use crate::actor::actor_selection::ActorSelectionMessage;
use crate::actor::behavior::Behavior;
use crate::actor::context::ActorContext;
use crate::actor::directive::Directive;
use crate::actor::receive::Receive;
use crate::actor_ref::ActorRef;
use crate::message::address_terminated::AddressTerminated;
use crate::message::identify::Identify;
use crate::message::kill::Kill;
use crate::message::poison_pill::PoisonPill;
use crate::message::terminated::Terminated;
use crate::message::{DynMessage, Message};

pub mod actor_selection;
pub mod actor_system;
pub mod address;
pub mod behavior;
pub mod context;
pub mod coordinated_shutdown;
pub mod dead_letter_listener;
pub mod directive;
pub mod extension;
pub(crate) mod mailbox;
pub mod props;
pub mod receive;
pub mod root_guardian;
pub mod scheduler;
pub(crate) mod state;
pub(crate) mod system_guardian;
pub mod timers;
pub(crate) mod user_guardian;
mod watching;

pub trait Actor: Send + Sized {
    #[allow(unused_variables)]
    fn started(&mut self, ctx: &mut ActorContext) -> anyhow::Result<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    fn stopped(&mut self, ctx: &mut ActorContext) -> anyhow::Result<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    fn on_child_failure(
        &mut self,
        context: &mut ActorContext,
        child: &ActorRef,
        error: &anyhow::Error,
    ) -> Directive {
        Directive::Resume
    }

    fn receive(&self) -> Receive<Self>;

    fn around_receive(
        &self,
        receive: &Receive<Self>,
        actor: &mut Self,
        ctx: &mut ActorContext,
        message: DynMessage,
        sender: Option<ActorRef>,
    ) -> anyhow::Result<Behavior<Self>> {
        receive.receive(actor, ctx, message, sender)
    }

    #[allow(unused_variables)]
    fn unhandled(&mut self, ctx: &mut ActorContext, message: DynMessage) {
        todo!("unhandled message: {:?}", message);
    }
}

pub(crate) fn is_auto_receive_message(message: &DynMessage) -> bool {
    static MSG: &'static [&'static str] = &[
        Terminated::signature_sized().name,
        AddressTerminated::signature_sized().name,
        PoisonPill::signature_sized().name,
        Kill::signature_sized().name,
        ActorSelectionMessage::signature_sized().name,
        Identify::signature_sized().name,
    ];

    if MSG.contains(&message.signature().name) {
        true
    } else {
        false
    }
}

pub(crate) fn is_system_message(message: &DynMessage) -> bool {
    todo!("is_system_message")
}
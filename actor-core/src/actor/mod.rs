use crate::actor::actor_selection::ActorSelectionMessage;
use crate::actor::behavior::Behavior;
use crate::actor::context::ActorContext;
use crate::actor::directive::Directive;
use crate::actor::receive::Receive;
use crate::actor_ref::ActorRef;
use crate::message::address_terminated::AddressTerminated;
use crate::message::death_watch_notification::DeathWatchNotification;
use crate::message::failed::Failed;
use crate::message::identify::Identify;
use crate::message::kill::Kill;
use crate::message::poison_pill::PoisonPill;
use crate::message::resume::Resume;
use crate::message::suspend::Suspend;
use crate::message::task_finish::TaskFinish;
use crate::message::terminate::Terminate;
use crate::message::terminated::Terminated;
use crate::message::unwatch::Unwatch;
use crate::message::watch::Watch;
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
pub(crate) mod watching;

pub trait Actor: Send + Sized {
    type Context: ActorContext;

    #[allow(unused_variables)]
    fn started(&mut self, ctx: &mut Self::Context) -> anyhow::Result<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    fn stopped(&mut self, ctx: &mut Self::Context) -> anyhow::Result<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    fn on_child_failure(
        &mut self,
        context: &mut Self::Context,
        child: &ActorRef,
        error: &anyhow::Error,
    ) -> Directive {
        Directive::Resume
    }

    fn receive(&self) -> Receive<Self>;

    fn around_receive(
        &mut self,
        receive: &Receive<Self>,
        ctx: &mut Self::Context,
        message: DynMessage,
        sender: Option<ActorRef>,
    ) -> anyhow::Result<Behavior<Self>> {
        receive.receive(self, ctx, message, sender)
    }

    #[allow(unused_variables)]
    fn unhandled(&mut self, ctx: &mut Self::Context, message: DynMessage) {
        todo!("unhandled message: {:?}", message);
    }
}

pub(crate) fn is_auto_received_message(message: &DynMessage) -> bool {
    let name = message.signature().name;
    if name == Terminated::signature_sized().name
        || name == AddressTerminated::signature_sized().name
        || name == PoisonPill::signature_sized().name
        || name == Kill::signature_sized().name
        || name == ActorSelectionMessage::signature_sized().name
        || name == Identify::signature_sized().name
        || name == TaskFinish::signature_sized().name
    {
        true
    } else {
        false
    }
}

pub(crate) fn is_system_message(message: &DynMessage) -> bool {
    let name = message.signature().name;
    if name == Watch::signature_sized().name
        || name == Terminate::signature_sized().name
        || name == Failed::signature_sized().name
        || name == Unwatch::signature_sized().name
        || name == DeathWatchNotification::signature_sized().name
        || name == Resume::signature_sized().name
        || name == Suspend::signature_sized().name
    {
        true
    } else {
        false
    }
}

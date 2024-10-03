use crate::actor::context::ActorContext;
use crate::actor::directive::Directive;
use crate::actor::receive::Receive;
use crate::actor_ref::ActorRef;

pub(crate) mod mailbox;
pub mod context;
pub(crate) mod state;
pub mod root_guardian;
pub(crate) mod system_guardian;
pub(crate) mod user_guardian;
pub mod timers;
pub mod actor_system;
pub mod address;
pub mod props;
pub mod actor_selection;
pub mod scheduler;
pub mod dead_letter_listener;
pub mod extension;
pub mod coordinated_shutdown;
pub mod directive;
mod watching;
mod receive;

pub trait Actor: Send + Sized {
    #[allow(unused_variables)]
    fn started(&mut self, ctx: &mut ActorContext<Self>) -> anyhow::Result<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    fn stopped(&mut self, ctx: &mut ActorContext<Self>) -> anyhow::Result<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    fn on_child_failure(&mut self, context: &mut ActorContext<Self>, child: &ActorRef, error: &anyhow::Error) -> Directive {
        Directive::Resume
    }

    fn receive(&self) -> Receive<Self>;
}

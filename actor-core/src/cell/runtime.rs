use std::any::type_name;
use std::collections::VecDeque;

use anyhow::anyhow;
use tokio::task::yield_now;
use tracing::{debug, error};

use crate::actor::actor_selection::ActorSelectionMessage;
use crate::actor::behavior::Behavior;
use crate::actor::context::{ActorContext, Context};
use crate::actor::mailbox::Mailbox;
use crate::actor::receive::Receive;
use crate::actor::state::ActorState;
use crate::actor::Actor;
use crate::actor_ref::actor_ref_factory::ActorRefFactory;
use crate::actor_ref::local_ref::SignalReceiver;
use crate::cell::envelope::Envelope;
use crate::message::address_terminated::AddressTerminated;
use crate::message::death_watch_notification::DeathWatchNotification;
use crate::message::failed::Failed;
use crate::message::identify::Identify;
use crate::message::kill::Kill;
use crate::message::poison_pill::PoisonPill;
use crate::message::resume::Resume;
use crate::message::suspend::Suspend;
use crate::message::terminate::Terminate;
use crate::message::terminated::Terminated;
use crate::message::unwatch::Unwatch;
use crate::message::watch::Watch;

pub struct ActorRuntime<A>
where
    A: Actor,
{
    pub(crate) actor: A,
    pub(crate) ctx: A::Context,
    pub(crate) mailbox: Mailbox,
    pub(crate) signal: SignalReceiver,
}

impl<A> ActorRuntime<A>
where
    A: Actor,
{
    pub(crate) async fn run(self) {
        let Self {
            mut actor,
            mut ctx,
            mut mailbox,
            mut signal,
        } = self;
        let mut behavior_stack = VecDeque::new();
        behavior_stack.push_back(actor.receive());
        let system_receive = Self::system_receive();

        signal.recv().await; //wait start signal

        if let Err(error) = actor.started(&mut ctx) {
            error!("actor {} start error {:?}", type_name::<A>(), error);
            ctx.stop(&ctx.myself());
            while let Some(message) = mailbox.system.recv() {
                Self::handle_system_message(&mut ctx, &mut actor, message, &system_receive);
                if matches!(ctx.state(), ActorState::CanTerminate) {
                    break;
                }
            }
            return;
        }
        ctx.context_mut().state = ActorState::Started;
        let mut throughput = 0;
        loop {
            tokio::select! {
                biased;
                Some(envelope) = mailbox.system.recv() => {
                    Self::handle_system_message(&mut ctx, &mut actor, envelope, &system_receive);
                    if matches!(ctx.state(), ActorState::CanTerminate) {
                        break;
                    }
                }
                Some(envelope) = mailbox.message.recv(), if matches!(ctx.state(), ActorState::Started) => {
                    Self::handle_message(&mut ctx, &mut actor, &mut behavior_stack, envelope);
                    throughput += 1;
                    if throughput >= mailbox.throughput {
                        throughput = 0;
                        yield_now().await;
                    }
                }
                else  => {
                    break;
                }
            }
        }
        if let Some(error) = actor.stopped(&mut ctx).err() {
            error!("actor {} stop error {:?}", type_name::<A>(), error);
        }
        mailbox.close();
        ctx.context_mut().state = ActorState::Terminated;
        for (name, handle) in ctx.context_mut().abort_handles {
            handle.abort();
            debug!("{} abort task: {}", type_name::<A>(), name);
        }
    }

    fn handle_message(
        ctx: &mut A::Context,
        actor: &mut A,
        behavior_stack: &mut VecDeque<Receive<A>>,
        envelope: Envelope,
    ) {
        let Envelope { message, sender } = envelope;
        let name = message.signature().name;
        match behavior_stack.get(0) {
            None => {
                actor.unhandled(ctx, message);
            }
            Some(receive) => {
                let behavior = std::panic::catch_unwind(|| {
                    actor.around_receive(receive, actor, ctx, message, sender)
                });
                match behavior {
                    Ok(behavior) => match behavior {
                        Ok(behavior) => match behavior {
                            Behavior::Same => {}
                            Behavior::Become {
                                receive,
                                discard_old,
                            } => {
                                if discard_old {
                                    behavior_stack.pop_front();
                                }
                                behavior_stack.push_front(receive);
                            }
                            Behavior::Unbecome => {
                                behavior_stack.pop_front();
                            }
                        },
                        Err(error) => {
                            ctx.handle_invoke_failure(name, error);
                        }
                    },
                    Err(_) => {
                        ctx.handle_invoke_failure(name, anyhow!("{} panic", name));
                    }
                }
            }
        }
    }

    fn handle_system_message(
        ctx: &mut Context,
        actor: &mut A,
        envelope: Envelope,
        receive: &Receive<A>,
    ) {
        let Envelope { message, sender } = envelope;
        let name = message.signature().name;
        if let Some(error) = receive.receive(actor, ctx, message, sender).err() {
            ctx.handle_invoke_failure(actor, name, error);
        }
    }

    fn handle_auto_receive_message(ctx: &mut Context, envelope: Envelope) {
        let Envelope { message, sender } = envelope;
    }

    fn system_receive() -> Receive<A> {
        Receive::new()
            .handle::<Failed>()
            .handle::<DeathWatchNotification>()
            .handle::<Watch>()
            .handle::<Unwatch>()
            .handle::<Suspend>()
            .handle::<Resume>()
            .handle::<Terminate>()
    }

    fn auto_receive() -> Receive<A> {
        Receive::new()
            .handle::<Terminated>()
            .handle::<AddressTerminated>()
            .handle::<Kill>()
            .handle::<PoisonPill>()
            .handle::<ActorSelectionMessage>()
            .handle::<Identify>()
    }
}

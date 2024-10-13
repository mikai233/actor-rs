use std::any::type_name;
use std::collections::VecDeque;
use std::panic::AssertUnwindSafe;

use anyhow::anyhow;
use tokio::task::yield_now;
use tracing::{debug, error};

use crate::actor::actor_selection::ActorSelectionMessage;
use crate::actor::behavior::Behavior;
use crate::actor::context::ActorContext;
use crate::actor::mailbox::Mailbox;
use crate::actor::receive::Receive;
use crate::actor::state::ActorState;
use crate::actor::{is_auto_received_message, Actor};
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
    pub(crate) signal_rx: SignalReceiver,
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
            mut signal_rx,
        } = self;
        let mut behavior_stack = VecDeque::new();
        behavior_stack.push_back(actor.receive());
        let system_receive = Self::system_receive();
        let auto_receive = Self::auto_receive();

        signal_rx.recv().await; //wait start signal

        if let Err(error) = actor.started(&mut ctx) {
            error!("actor `{}` start error {:?}", type_name::<A>(), error);
            ctx.stop(ctx.context().myself());
            while let Some(message) = mailbox.system.recv().await {
                Self::receive(&mut ctx, &mut actor, message, &system_receive);
                if matches!(ctx.context().state, ActorState::CanTerminate) {
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
                    Self::receive(&mut ctx, &mut actor, envelope, &system_receive);
                    if matches!(ctx.context().state, ActorState::CanTerminate) {
                        break;
                    }
                }
                Some(envelope) = mailbox.message.recv(), if matches!(ctx.context().state, ActorState::Started) => {
                    if is_auto_received_message(&envelope.message) {
                        Self::receive(&mut ctx, &mut actor,envelope,&auto_receive);
                    } else {
                        Self::handle_message(&mut ctx, &mut actor, &mut behavior_stack, envelope);
                    }
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
            error!("actor `{}` stop error {:?}", type_name::<A>(), error);
        }
        mailbox.close();
        let context = ctx.context_mut();
        context.state = ActorState::Terminated;
        for (name, handle) in &context.abort_handles {
            handle.abort();
            debug!("actor `{}` abort task: {}", type_name::<A>(), name);
        }
        context.abort_handles.clear();
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
                let behavior = std::panic::catch_unwind(AssertUnwindSafe(|| {
                    actor.around_receive(receive, ctx, message, sender)
                }));
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
                            ctx.context_mut()
                                .handle_invoke_failure(type_name::<A>(), name, error);
                        }
                    },
                    Err(_) => {
                        ctx.context_mut().handle_invoke_failure(
                            type_name::<A>(),
                            name,
                            anyhow!("{} panic", name),
                        );
                    }
                }
            }
        }
    }

    fn receive(
        ctx: &mut A::Context,
        actor: &mut A,
        envelope: Envelope,
        receive: &Receive<A>,
    ) {
        let Envelope { message, sender } = envelope;
        let name = message.signature().name;
        if let Some(error) = receive.receive(actor, ctx, message, sender).err() {
            ctx.context_mut()
                .handle_invoke_failure(type_name::<A>(), name, error);
        }
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

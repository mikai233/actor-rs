use std::any::type_name;
use std::any::Any;
use std::collections::VecDeque;
use std::panic::AssertUnwindSafe;

use anyhow::anyhow;
use futures::FutureExt;
use tokio::task::yield_now;
use tracing::{debug, error};

use crate::actor::behavior::Behavior;
use crate::actor::context::{ActorContext, Context};
use crate::actor::mailbox::Mailbox;
use crate::actor::receive::{Receive, ReceiveFn};
use crate::actor::state::ActorState;
use crate::actor::Actor;
use crate::actor_ref::actor_ref_factory::ActorRefFactory;
use crate::cell::envelope::Envelope;

pub struct ActorRuntime<A> where A: Actor {
    pub(crate) actor: A,
    pub(crate) context: ActorContext<A>,
    pub(crate) mailbox: Mailbox,
}

impl<A> ActorRuntime<A> where A: Actor {
    pub(crate) async fn run(self) {
        let Self { mut actor, mut context, mut mailbox } = self;
        context.stash_capacity = mailbox.stash_capacity;
        let mut behavior_stack = VecDeque::new();
        behavior_stack.push_back(actor.receive());
        if let Err(error) = actor.started(&mut context) {
            error!("actor {} start error {:?}", type_name::<A>(), error);
            context.stop(&context.myself());
            while let Some(message) = mailbox.system.recv() {
                Self::handle_message(&mut context, &mut actor, &mut behavior_stack, message);
                if matches!(context.state, ActorState::CanTerminate) {
                    break;
                }
            }
            return;
        }
        context.state = ActorState::Started;
        let mut throughput = 0;
        loop {
            tokio::select! {
                biased;
                Some(envelope) = mailbox.system.recv() => {
                    Self::handle_message(&mut context, &mut actor, &mut behavior_stack, envelope);
                    if matches!(context.state, ActorState::CanTerminate) {
                        break;
                    }
                }
                Some(envelope) = mailbox.message.recv(), if matches!(context.state, ActorState::Started) => {
                    Self::handle_message(&mut context, &mut actor, &mut behavior_stack, envelope);
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
        if let Some(error) = actor.stopped(&mut context).err() {
            error!("actor {} stop error {:?}", type_name::<A>(), error);
        }
        mailbox.close();
        context.state = ActorState::Terminated;
        for (name, handle) in context.abort_handles {
            handle.abort();
            debug!("{} abort task: {}", type_name::<A>(), name);
        }
    }

    fn handle_message(
        ctx: &mut ActorContext<A>,
        actor: &mut A,
        behavior_stack: &mut VecDeque<Receive<A>>,
        envelope: Envelope,
    ) {
        let Envelope { message, sender } = envelope;
        match behavior_stack.get(0) {
            None => {
                actor.unhandled(ctx, message);
            }
            Some(receive) => {
                let behavior = std::panic::catch_unwind(|| {
                    receive(actor, ctx, message, sender)
                });
                match behavior {
                    Ok(behavior) => {
                        match behavior {
                            Ok(behavior) => {
                                match behavior {
                                    Behavior::Same => {}
                                    Behavior::Become { receive, discard_old } => {
                                        if discard_old {
                                            behavior_stack.pop_front();
                                        }
                                        behavior_stack.push_front(receive);
                                    }
                                    Behavior::Unbecome => {
                                        behavior_stack.pop_front();
                                    }
                                }
                            }
                            Err(error) => {
                                ctx.handle_invoke_failure(message.signature().name, error);
                            }
                        }
                    }
                    Err(_) => {
                        ctx.handle_invoke_failure(message.signature().name, anyhow!("{} panic", message.signature().name));
                    }
                }
            }
        }
    }
}
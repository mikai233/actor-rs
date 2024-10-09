use std::any::type_name;
use std::any::Any;
use std::collections::VecDeque;
use std::ops::Deref;
use std::panic::AssertUnwindSafe;

use anyhow::anyhow;
use futures::FutureExt;
use tokio::task::yield_now;
use tracing::{debug, error};

use crate::actor::behavior::Behavior;
use crate::actor::context::{ActorContext, Context};
use crate::actor::directive::Directive;
use crate::actor::mailbox::Mailbox;
use crate::actor::receive::{Receive, ReceiveFn};
use crate::actor::state::ActorState;
use crate::actor::Actor;
use crate::actor_ref::actor_ref_factory::ActorRefFactory;
use crate::actor_ref::local_ref::SignalReceiver;
use crate::actor_ref::{ActorRef, ActorRefExt};
use crate::cell::envelope::Envelope;
use crate::message::failed::Failed;
use crate::message::watch::Watch;

pub struct ActorRuntime<A> where A: Actor {
    pub(crate) actor: A,
    pub(crate) ctx: A::Context,
    pub(crate) mailbox: Mailbox,
    pub(crate) signal: SignalReceiver,
}

impl<A> ActorRuntime<A> where A: Actor {
    pub(crate) async fn run(self) {
        let Self { mut actor, mut ctx, mut mailbox, mut signal } = self;
        let mut behavior_stack = VecDeque::new();
        behavior_stack.push_back(actor.receive());

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
                    receive.receive(actor, ctx, message, sender)
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
                                ctx.handle_invoke_failure(name, error);
                            }
                        }
                    }
                    Err(_) => {
                        ctx.handle_invoke_failure(name, anyhow!("{} panic", name));
                    }
                }
            }
        }
    }

    fn handle_system_message(
        ctx: &mut Context,
        envelope: Envelope,
    ) {
        let Envelope { message, sender } = envelope;
        let name = message.signature().name;
        if let Some(error) = system_receive.receive(actor, ctx, message, sender).err() {
            ctx.handle_invoke_failure(name, error);
        }
    }

    fn handle_failure(actor: &mut A, ctx: &mut Context, msg: Failed, _: Option<ActorRef>) -> anyhow::Result<Behavior<A>> {
        let Failed { child, error } = msg;
        let directive = actor.on_child_failure(ctx, &child, &error);
        match directive {
            Directive::Resume => {
                child.resume();
            }
            Directive::Stop => {
                debug_assert!(ctx.children().iter().find(|child| child == &&child).is_some());
                ctx.stop(&child);
            }
            Directive::Escalate => {
                if let Some(parent) = ctx.parent() {
                    parent.cast_ns(Failed { child, error });
                }
            }
        }
        Ok(Behavior::same())
    }

    fn add_watcher(ctx: &mut Context, msg: Watch, _: Option<ActorRef>) -> anyhow::Result<Behavior<A>> {
        let Watch { watchee, watcher } = msg;
        let watchee_self = watchee == ctx.myself;
        let watcher_self = watcher == ctx.myself;
        if watchee_self && !watcher_self {
            if !ctx.watched_by.contains(&watcher) {
                ctx.maintain_address_terminated_subscription(Some(&watcher), |ctx| {
                    debug!("{} is watched by {}", ctx.myself, watcher);
                    ctx.watched_by.insert(watcher.clone());
                });
            } else {
                debug!("watcher {} already added for {}", watcher, ctx.myself());
            }
        } else {
            error!("illegal Watch({},{}) for {}", watchee, watcher, ctx.myself());
        }
        Ok(Behavior::same())
    }
}
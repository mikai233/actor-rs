use async_trait::async_trait;
use futures::future::{BoxFuture, join_all};
use tracing::{debug, info};

use actor_derive::EmptyCodec;

use crate::{Actor, AsyncMessage, CodecMessage, Message};
use crate::actor_ref::{ActorRef, TActorRef};
use crate::context::{ActorContext, Context};
use crate::message::terminated::WatchTerminated;
use crate::provider::ActorRefFactory;

#[derive(Debug)]
pub(crate) struct RootGuardian;

#[derive(Default)]
pub(crate) struct State {
    shutdown_hooks: Vec<BoxFuture<'static, ()>>,
    user_stopped: bool,
    system_stopped: bool,
    shutdown_signal: Option<tokio::sync::oneshot::Sender<()>>,
}


#[async_trait]
impl Actor for RootGuardian {
    type S = State;
    type A = ();

    async fn pre_start(&self, context: &mut ActorContext, _arg: Self::A) -> anyhow::Result<Self::S> {
        debug!("{} pre start", context.myself());
        Ok(State::default())
    }

    async fn post_stop(&self, context: &mut ActorContext, state: &mut Self::S) -> anyhow::Result<()> {
        debug!("{} post stop", context.myself());
        if let Some(signal) = state.shutdown_signal.take() {
            let _ = signal.send(());
        }
        Ok(())
    }
}

#[derive(Debug, EmptyCodec)]
pub(crate) struct Shutdown {
    pub(crate) signal: tokio::sync::oneshot::Sender<()>,
}

#[async_trait]
impl AsyncMessage for Shutdown {
    type T = RootGuardian;

    async fn handle(self: Box<Self>, context: &mut ActorContext, state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
        debug!("shutdown received, start shutdown actor system");
        state.shutdown_signal = Some(self.signal);
        join_all(state.shutdown_hooks.drain(..)).await;
        context.system.guardian().stop();
        Ok(())
    }
}

#[derive(Debug, EmptyCodec)]
pub(crate) struct ChildGuardianStarted {
    pub(crate) guardian: ActorRef,
}

impl Message for ChildGuardianStarted {
    type T = RootGuardian;

    fn handle(self: Box<Self>, context: &mut ActorContext, _state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
        context.watch(WatchGuardianTerminated { guardian: self.guardian });
        Ok(())
    }
}

#[derive(Debug, EmptyCodec)]
pub(crate) struct WatchGuardianTerminated {
    guardian: ActorRef,
}

impl WatchTerminated for WatchGuardianTerminated {
    fn watch_actor(&self) -> &ActorRef {
        &self.guardian
    }
}

impl Message for WatchGuardianTerminated {
    type T = RootGuardian;

    fn handle(self: Box<Self>, context: &mut ActorContext, state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
        if self.guardian == context.system.guardian().into() {
            state.user_stopped = true;
            debug!("user guardian stopped");
            match context.children().get("system") {
                None => {
                    state.system_stopped = true;
                }
                Some(system) => {
                    system.stop();
                }
            }
        } else if self.guardian == context.system.system_guardian().into() {
            state.system_stopped = true;
            debug!("system guardian stopped");
        }
        if state.user_stopped && state.system_stopped {
            debug!("user guardian and system guardian stopped, stop the root guardian");
            context.stop(context.myself());
        }
        Ok(())
    }
}

#[derive(EmptyCodec)]
pub(crate) struct AddShutdownHook {
    pub(crate) fut: BoxFuture<'static, ()>,
}

impl Message for AddShutdownHook {
    type T = RootGuardian;

    fn handle(self: Box<Self>, context: &mut ActorContext, state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
        state.shutdown_hooks.push(self.fut);
        Ok(())
    }
}
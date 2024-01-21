use async_trait::async_trait;
use futures::future::{BoxFuture, join_all};
use tracing::debug;

use actor_derive::EmptyCodec;

use crate::{Actor, Message};
use crate::actor::actor_ref::{ActorRef, TActorRef};
use crate::actor::actor_ref_factory::ActorRefFactory;
use crate::actor::context::{ActorContext, Context};
use crate::message::terminated::WatchTerminated;

#[derive(Default)]
pub(crate) struct RootGuardian {
    shutdown_hooks: Vec<BoxFuture<'static, ()>>,
    user_stopped: bool,
    system_stopped: bool,
    shutdown_signal: Option<tokio::sync::oneshot::Sender<()>>,
}

#[async_trait]
impl Actor for RootGuardian {
    async fn started(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        debug!("{} pre start", context.myself());
        Ok(())
    }

    async fn stopped(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        debug!("{} post stop", context.myself());
        if let Some(signal) = self.shutdown_signal.take() {
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
impl Message for Shutdown {
    type A = RootGuardian;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        debug!("shutdown received, start shutdown actor system");
        actor.shutdown_signal = Some(self.signal);
        join_all(actor.shutdown_hooks.drain(..)).await;
        context.system.guardian().stop();
        Ok(())
    }
}

#[derive(Debug, EmptyCodec)]
pub(crate) struct ChildGuardianStarted {
    pub(crate) guardian: ActorRef,
}

#[async_trait]
impl Message for ChildGuardianStarted {
    type A = RootGuardian;

    async fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
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

#[async_trait]
impl Message for WatchGuardianTerminated {
    type A = RootGuardian;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        if self.guardian == context.system.guardian().into() {
            actor.user_stopped = true;
            debug!("user guardian stopped");
            match context.child("system") {
                None => {
                    actor.system_stopped = true;
                }
                Some(system) => {
                    system.stop();
                }
            }
        } else if self.guardian == context.system.system_guardian().into() {
            actor.system_stopped = true;
            debug!("system guardian stopped");
        }
        if actor.user_stopped && actor.system_stopped {
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

#[async_trait]
impl Message for AddShutdownHook {
    type A = RootGuardian;

    async fn handle(self: Box<Self>, _context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        actor.shutdown_hooks.push(self.fut);
        Ok(())
    }
}
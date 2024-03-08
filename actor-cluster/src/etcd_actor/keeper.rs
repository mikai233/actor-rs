use std::time::Duration;

use async_trait::async_trait;

use actor_core::{Actor, Message};
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::actor_ref::ActorRefExt;
use actor_core::ext::option_ext::OptionExt;
use actor_derive::EmptyCodec;

use crate::etcd_actor::keeper_keep_alive_failed::KeeperKeepAliveFailed;

#[derive(Debug)]
pub(super) struct Keeper {
    pub(super) keeper: etcd_client::LeaseKeeper,
    pub(super) interval: Duration,
}

#[async_trait]
impl Actor for Keeper {
    async fn started(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        let myself = context.myself().clone();
        context.system().scheduler().schedule_with_fixed_delay(None, self.interval, move || {
            myself.cast_ns(KeepAliveTick);
        });
        Ok(())
    }
}

#[derive(Debug, Clone, EmptyCodec)]
struct KeepAliveTick;

#[async_trait]
impl Message for KeepAliveTick {
    type A = Keeper;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        if let Some(error) = actor.keeper.keep_alive().await.err() {
            context.parent().foreach(|parent| {
                parent.cast_ns(KeeperKeepAliveFailed {
                    id: actor.keeper.id(),
                    error,
                });
            });
            context.stop(context.myself());
        }
        Ok(())
    }
}
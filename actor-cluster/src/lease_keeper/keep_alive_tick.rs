use std::time::Duration;

use async_trait::async_trait;
use tracing::warn;

use actor_core::{DynMessage, Message};
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::actor_ref::ActorRef;
use actor_derive::CEmptyCodec;

use crate::lease_keeper::EtcdLeaseKeeper;
use crate::lease_keeper::lease_keep_alive_failed::LeaseKeepAliveFailed;

#[derive(Debug, Clone, CEmptyCodec)]
pub(super) struct KeepAliveTick;

#[async_trait]
impl Message for KeepAliveTick {
    type A = EtcdLeaseKeeper;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let keeper = actor.keeper.as_mut().unwrap();
        if let Some(error) = keeper.keep_alive().await.err() {
            actor.failed_receiver.tell(DynMessage::orphan(LeaseKeepAliveFailed(keeper.id())), ActorRef::no_sender());
            warn!("{} {} lease keep alive failed {:?}, stop the lease keeper", context.myself(), keeper.id(), error);
            context.stop(context.myself());
        } else {
            let stream = actor.stream.as_mut().unwrap();
            match tokio::time::timeout(Duration::from_secs(3), stream.message()).await {
                Ok(resp) => {
                    match resp {
                        Ok(_) => {}
                        Err(error) => {
                            warn!("{} wait lease response error {:?}", context.myself(), error);
                        }
                    }
                }
                Err(_) => {
                    warn!("{} wait lease response timeout", context.myself());
                }
            }
        }
        Ok(())
    }
}
use std::time::Duration;

use async_trait::async_trait;

use actor_core::{DynMessage, Message};
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::props::Props;
use actor_core::actor_ref::{ActorRef, ActorRefExt};
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_derive::{EmptyCodec, OrphanEmptyCodec};

use crate::etcd_actor::EtcdActor;
use crate::etcd_actor::keeper::Keeper;
use crate::etcd_actor::lease::Lease;
use crate::etcd_actor::poll_keep_alive_resp::PollKeepAliveResp;

#[derive(Debug, EmptyCodec)]
pub struct KeepAlive {
    pub id: i64,
    pub watcher: ActorRef,
    pub interval: Duration,
}

#[async_trait]
impl Message for KeepAlive {
    type A = EtcdActor;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        match actor.client.lease_keep_alive(self.id).await {
            Ok((keeper, stream)) => {
                let interval = self.interval;
                let keeper = context.spawn_anonymous(Props::new(move || { Ok(Keeper { keeper, interval }) }))?;
                let lease = Lease {
                    keeper,
                    stream,
                    watcher: self.watcher,
                };
                actor.lease.insert(self.id, lease);
                context.myself().cast_ns(PollKeepAliveResp);
            }
            Err(error) => {
                let keep_alive_failed = KeepAliveFailed {
                    id: self.id,
                    error: Some(error),
                };
                self.watcher.tell(DynMessage::orphan(keep_alive_failed), ActorRef::no_sender());
            }
        }
        Ok(())
    }
}

#[derive(Debug, OrphanEmptyCodec)]
pub struct KeepAliveFailed {
    pub id: i64,
    pub error: Option<etcd_client::Error>,
}
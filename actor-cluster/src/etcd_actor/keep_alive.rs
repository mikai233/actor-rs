use std::time::Duration;

use async_trait::async_trait;
use tracing::{debug, error};

use actor_core::{EmptyCodec, OrphanEmptyCodec};
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::props::Props;
use actor_core::actor_ref::{ActorRef, ActorRefExt};
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::Message;

use crate::etcd_actor::EtcdActor;
use crate::etcd_actor::keeper::Keeper;
use crate::etcd_actor::lease::Lease;
use crate::etcd_actor::poll_keep_alive_resp::PollKeepAliveResp;

#[derive(Debug, EmptyCodec)]
pub struct KeepAlive {
    pub id: i64,
    pub applicant: ActorRef,
    pub interval: Duration,
}

#[async_trait]
impl Message for KeepAlive {
    type A = EtcdActor;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> eyre::Result<()> {
        debug!("{} request keep alive {} with interval {:?}", self.applicant, self.id, self.interval);
        match actor.client.lease_keep_alive(self.id).await {
            Ok((keeper, stream)) => {
                let interval = self.interval;
                match context.spawn_anonymous(Props::new(move || { Ok(Keeper { keeper, interval, tick_key: None }) })) {
                    Ok(keeper) => {
                        let lease = Lease {
                            keeper,
                            stream,
                            applicant: self.applicant,
                        };
                        actor.lease.insert(self.id, lease);
                        context.myself().cast_ns(PollKeepAliveResp);
                    }
                    Err(error) => {
                        error!("spawn lease keeper {} failed: {:?}", self.id, error);
                        EtcdActor::keep_alive_failed(self.id, &self.applicant, None);
                    }
                };
            }
            Err(error) => {
                EtcdActor::keep_alive_failed(self.id, &self.applicant, Some(error));
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
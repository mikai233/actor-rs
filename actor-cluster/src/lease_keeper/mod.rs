use std::time::Duration;

use async_trait::async_trait;

use actor_core::{Actor, DynMessage};
use actor_core::actor::actor_ref_factory::ActorRefFactory;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::scheduler::ScheduleKey;
use actor_core::actor_ref::{ActorRef, ActorRefExt};
use actor_core::ext::etcd_client::EtcdClient;
use actor_core::ext::option_ext::OptionExt;

use crate::lease_keeper::keep_alive_tick::KeepAliveTick;
use crate::lease_keeper::lease_keep_alive_failed::LeaseKeepAliveFailed;

pub mod lease_keep_alive_failed;
mod keep_alive_tick;
mod lease_revoke;

pub struct EtcdLeaseKeeper {
    client: EtcdClient,
    lease_id: i64,
    keeper: Option<etcd_client::LeaseKeeper>,
    stream: Option<etcd_client::LeaseKeepAliveStream>,
    failed_receiver: ActorRef,
    interval: Duration,
    tick_key: Option<ScheduleKey>,
}

impl EtcdLeaseKeeper {
    pub fn new(client: EtcdClient, lease_id: i64, failed_receiver: ActorRef, interval: Duration) -> Self {
        Self {
            client,
            lease_id,
            keeper: None,
            stream: None,
            failed_receiver,
            interval,
            tick_key: None,
        }
    }
}

#[async_trait]
impl Actor for EtcdLeaseKeeper {
    async fn started(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        match self.client.lease_keep_alive(self.lease_id).await {
            Ok((keeper, stream)) => {
                self.keeper = Some(keeper);
                self.stream = Some(stream);
            }
            Err(error) => {
                self.failed_receiver.tell(DynMessage::orphan(LeaseKeepAliveFailed(self.lease_id)), ActorRef::no_sender());
                return Err(anyhow::Error::from(error));
            }
        };
        let myself = context.myself().clone();
        let key = context.system()
            .scheduler()
            .schedule_with_fixed_delay(None, self.interval, move || {
                myself.cast_ns(KeepAliveTick);
            });
        self.tick_key = Some(key);
        Ok(())
    }

    async fn stopped(&mut self, _context: &mut ActorContext) -> anyhow::Result<()> {
        let _ = self.client.lease_revoke(self.lease_id).await;
        self.tick_key.take().into_foreach(|k| { k.cancel(); });
        Ok(())
    }
}

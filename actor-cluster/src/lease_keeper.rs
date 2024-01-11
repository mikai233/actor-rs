use std::time::Duration;

use async_trait::async_trait;
use etcd_client::Client;
use tracing::{debug, warn};

use actor_core::{Actor, DynMessage, Message};
use actor_core::actor::actor_ref::ActorRef;
use actor_core::actor::actor_ref_factory::ActorRefFactory;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::timer_scheduler::ScheduleKey;
use actor_core::ext::option_ext::OptionExt;
use actor_derive::{EmptyCodec, OrphanEmptyCodec};

pub(crate) struct LeaseKeeper {
    eclient: Client,
    lease_id: i64,
    keeper: Option<etcd_client::LeaseKeeper>,
    stream: Option<etcd_client::LeaseKeepAliveStream>,
    failed_receiver: ActorRef,
    interval: Duration,
    tick_key: Option<ScheduleKey>,
}

impl LeaseKeeper {
    pub fn new(eclient: Client, lease_id: i64, failed_receiver: ActorRef, interval: Duration) -> Self {
        Self {
            eclient,
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
impl Actor for LeaseKeeper {
    async fn pre_start(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        match self.eclient.lease_keep_alive(self.lease_id).await {
            Ok((keeper, stream)) => {
                self.keeper = Some(keeper);
                self.stream = Some(stream);
            }
            Err(error) => {
                self.failed_receiver.tell(DynMessage::orphan(LeaseKeepAliveFailed(self.lease_id)), ActorRef::no_sender());
                return Err(anyhow::Error::from(error));
            }
        };
        let tick_key = context.system().scheduler().start_timer_with_fixed_delay(None, self.interval, DynMessage::user(LeaseTick), context.myself().clone());
        self.tick_key = Some(tick_key);
        Ok(())
    }

    async fn post_stop(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        self.tick_key.take().into_foreach(|k| { k.cancel(); });
        Ok(())
    }
}

#[derive(Debug, EmptyCodec)]
struct RevokeLease;

#[async_trait]
impl Message for RevokeLease {
    type A = LeaseKeeper;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let keeper = actor.keeper.as_ref().unwrap();
        actor.eclient.lease_revoke(keeper.id()).await?;
        debug!("{} {} lease revoke success", context.myself(), keeper.id());
        Ok(())
    }
}

#[derive(Debug, EmptyCodec)]
struct LeaseTick;

#[async_trait]
impl Message for LeaseTick {
    type A = LeaseKeeper;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let mut keeper = actor.keeper.as_mut().unwrap();
        if let Some(error) = keeper.keep_alive().await.err() {
            actor.failed_receiver.tell(DynMessage::orphan(LeaseKeepAliveFailed(keeper.id())), ActorRef::no_sender());
            warn!("{} {} lease keep alive failed {:?}", context.myself(), keeper.id(), error);
            context.stop(context.myself());
        } else {
            let mut stream = actor.stream.as_mut().unwrap();
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

#[derive(Debug, OrphanEmptyCodec)]
pub struct LeaseKeepAliveFailed(i64);

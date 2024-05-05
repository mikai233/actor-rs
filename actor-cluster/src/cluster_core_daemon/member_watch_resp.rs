use std::time::Duration;

use async_trait::async_trait;
use etcd_client::{EventType, WatchResponse};
use tracing::warn;

use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::actor_ref::ActorRefExt;
use actor_core::EmptyCodec;
use actor_core::Message;

use crate::cluster_core_daemon::ClusterCoreDaemon;
use crate::cluster_core_daemon::self_removed::SelfRemoved;
use crate::cluster_core_daemon::watch_failed::WatchFailed;
use crate::cluster_event::ClusterEvent;
use crate::etcd_actor::watch::WatchResp;
use crate::member::MemberStatus;

const WATCH_RETRY_DELAY: Duration = Duration::from_secs(3);

#[derive(Debug, EmptyCodec)]
pub(crate) struct MemberWatchResp(pub(crate) WatchResp);

#[async_trait]
impl Message for MemberWatchResp {
    type A = ClusterCoreDaemon;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> eyre::Result<()> {
        match self.0 {
            WatchResp::Update(resp) => {
                Self::update_member_status(context, actor, resp).await?;
            }
            WatchResp::Failed(error) => {
                match error {
                    None => {
                        warn!("{} watch members status error, try rewatch it", context.myself());
                    }
                    Some(error) => {
                        warn!("{} watch members status error {:?}, try rewatch it", context.myself(), error);
                    }
                }
                let myself = context.myself().clone();
                context.system().scheduler.schedule_once(WATCH_RETRY_DELAY, move || {
                    myself.cast_ns(WatchFailed);
                });
            }
            WatchResp::Started => {
                match actor.get_all_members(context).await {
                    Ok(_) => {
                        actor.try_keep_alive(context).await;
                    }
                    Err(error) => {
                        warn!("get members form etcd error {:?}", error);
                        let myself = context.myself().clone();
                        context.system().scheduler.schedule_once(WATCH_RETRY_DELAY, move || {
                            myself.cast_ns(WatchFailed);
                        });
                    }
                }
            }
        }
        Ok(())
    }
}

impl MemberWatchResp {
    /// update local member status form etcd
    async fn update_member_status(context: &mut ActorContext, actor: &mut ClusterCoreDaemon, resp: WatchResponse) -> eyre::Result<()> {
        for event in resp.events() {
            if let Some(kv) = event.kv() {
                match event.event_type() {
                    EventType::Put => {
                        actor.update_local_member_status(context, kv)?;
                    }
                    EventType::Delete => {
                        if let Some(addr) = actor.key_addr.remove(kv.key_str()?) {
                            let stream = &context.system().event_stream;
                            if let Some(mut member) = actor.cluster.members_write().remove(&addr) {
                                member.status = MemberStatus::Removed;
                                if addr == actor.self_addr {
                                    context.myself().cast_ns(SelfRemoved);
                                }
                                stream.publish(ClusterEvent::member_removed(member.clone()));
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
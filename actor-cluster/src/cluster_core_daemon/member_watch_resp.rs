use async_trait::async_trait;
use etcd_client::{EventType, WatchResponse};
use tracing::{debug, error};

use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::actor_ref::ActorRefExt;
use actor_core::Message;
use actor_derive::EmptyCodec;

use crate::cluster_core_daemon::ClusterCoreDaemon;
use crate::cluster_core_daemon::self_removed::SelfRemoved;
use crate::cluster_event::ClusterEvent;
use crate::etcd_actor::watch::WatchResp;
use crate::member::MemberStatus;

#[derive(Debug, EmptyCodec)]
pub(crate) struct MemberWatchResp(pub(crate) WatchResp);

#[async_trait]
impl Message for MemberWatchResp {
    type A = ClusterCoreDaemon;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        match self.0 {
            WatchResp::Success(resp) => {
                debug!("watch resp {:?}", resp);
                Self::update_member_status(context, actor, resp).await?;
            }
            WatchResp::Failed(error) => {
                match error {
                    None => {
                        error!("{} watch members status error, try rewatch it", context.myself());
                    }
                    Some(error) => {
                        error!("{} watch members status error {:?}, try rewatch it", context.myself(), error);
                    }
                }
                actor.watch_cluster_members();
            }
        }
        Ok(())
    }
}

impl MemberWatchResp {
    /// update local member status form etcd
    async fn update_member_status(context: &mut ActorContext, actor: &mut ClusterCoreDaemon, resp: WatchResponse) -> anyhow::Result<()> {
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
                                stream.publish(ClusterEvent::member_removed(member.clone()))?;
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
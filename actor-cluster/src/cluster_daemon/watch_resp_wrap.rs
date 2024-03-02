use async_trait::async_trait;
use etcd_client::{EventType, WatchResponse};

use actor_core::{DynMessage, Message};
use actor_core::actor::actor_ref::ActorRefExt;
use actor_core::actor::actor_ref_factory::ActorRefFactory;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::event::EventBus;
use actor_derive::EmptyCodec;

use crate::cluster_daemon::ClusterDaemon;
use crate::cluster_daemon::self_down::SelfDown;
use crate::cluster_daemon::self_removed::SelfRemoved;
use crate::cluster_event::ClusterEvent;
use crate::etcd_watcher::watch_resp::WatchResp;
use crate::member::MemberStatus;

#[derive(Debug, EmptyCodec)]
pub(super) struct WatchRespWrap(pub(super) WatchResp);

#[async_trait]
impl Message for WatchRespWrap {
    type A = ClusterDaemon;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let WatchResp { key, resp } = self.0;
        if key == actor.lease_path() {
            Self::update_member_status(context, actor, resp).await?;
        }
        Ok(())
    }
}

impl WatchRespWrap {
    /// update local member status form etcd
    async fn update_member_status(context: &mut ActorContext, actor: &mut ClusterDaemon, resp: WatchResponse) -> anyhow::Result<()> {
        for event in resp.events() {
            if let Some(kv) = event.kv() {
                match event.event_type() {
                    EventType::Put => {
                        actor.update_local_member_status(context, kv)?;
                    }
                    EventType::Delete => {
                        if let Some(addr) = actor.key_addr.remove(kv.key_str()?) {
                            let cluster = actor.cluster.as_ref().unwrap();
                            let stream = context.system().event_stream();
                            if let Some(mut member) = cluster.members_write().remove(&addr) {
                                member.status = MemberStatus::Removed;
                                if addr == actor.self_addr {
                                    context.myself().cast_ns(SelfRemoved);
                                }
                                stream.publish(DynMessage::orphan(ClusterEvent::member_removed(member.clone())))?;
                                member.status = MemberStatus::Down;
                                if addr == actor.self_addr {
                                    context.myself().cast_ns(SelfDown);
                                }
                                stream.publish(DynMessage::orphan(ClusterEvent::member_downed(member)))?;
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
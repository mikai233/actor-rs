use ahash::HashMap;
use async_trait::async_trait;
use tracing::debug;

use actor_cluster::cluster_event::ClusterEvent;
use actor_cluster::member::MemberStatus;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::EmptyCodec;
use actor_core::Message;

use crate::singleton::cluster_singleton_proxy::ClusterSingletonProxy;

#[derive(Debug, EmptyCodec)]
pub(super) struct ClusterEventWrap(pub(super) ClusterEvent);

#[async_trait]
impl Message for ClusterEventWrap {
    type A = ClusterSingletonProxy;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        match self.0 {
            ClusterEvent::MemberUp(m) => {
                debug!("member up {}", m);
                if actor.matching_role(&m) {
                    actor.host_singleton_members.insert(m.unique_address.clone(), m);
                    actor.identify_singleton(context);
                }
            }
            ClusterEvent::MemberPrepareForLeaving(_) => {}
            ClusterEvent::MemberLeaving(_) => {}
            ClusterEvent::MemberRemoved(m) => {
                debug!("member removed {}", m);
                if m.unique_address == actor.cluster.self_member().unique_address {
                    context.stop(context.myself());
                } else if actor.matching_role(&m) {
                    actor.host_singleton_members.remove(&m.unique_address);
                    //TODO 或许只需要观察到Singleton terminated的时候才需要执行identify_singleton ?
                    actor.identify_singleton(context);
                }
            }
            ClusterEvent::CurrentClusterState { members, .. } => {
                let host_members = members
                    .into_iter()
                    .filter(|(_, m)| { m.status == MemberStatus::Up && actor.matching_role(m) })
                    .collect::<HashMap<_, _>>();
                actor.host_singleton_members.extend(host_members);
                actor.identify_singleton(context);
            }
            ClusterEvent::EtcdUnreachable => {}
        }
        Ok(())
    }
}
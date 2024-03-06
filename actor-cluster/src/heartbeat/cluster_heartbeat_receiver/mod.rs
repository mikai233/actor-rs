use async_trait::async_trait;
use tracing::trace;

use actor_core::{Actor, DynMessage};
use actor_core::actor::actor_ref_factory::ActorRefFactory;
use actor_core::actor::address::Address;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor_path::{ActorPath, TActorPath};
use actor_core::actor_path::root_actor_path::RootActorPath;
use actor_core::actor_ref::ActorRef;

use crate::cluster::Cluster;
use crate::heartbeat::cluster_heartbeat_receiver::heartbeat_receiver_cluster_event::HeartbeatReceiverClusterEvent;
use crate::member::Member;

pub(crate) mod heartbeat;
mod heartbeat_receiver_cluster_event;

#[derive(Debug)]
pub(crate) struct ClusterHeartbeatReceiver {
    self_member: Option<Member>,
    event_adapter: ActorRef,
}

#[async_trait]
impl Actor for ClusterHeartbeatReceiver {
    async fn started(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        trace!("started {}", context.myself());
        Cluster::get(context.system()).subscribe_cluster_event(self.event_adapter.clone());
        Ok(())
    }
}

impl ClusterHeartbeatReceiver {
    pub(crate) fn new(context: &mut ActorContext) -> Self {
        let event_adapter = context.message_adapter(|m| DynMessage::user(HeartbeatReceiverClusterEvent(m)));
        Self {
            self_member: None,
            event_adapter,
        }
    }

    pub(crate) fn name() -> &'static str {
        "heartbeat_receiver"
    }

    pub(crate) fn path(address: Address) -> ActorPath {
        RootActorPath::new(address, "/").descendant(vec!["system", "cluster", Self::name()]).into()
    }
}
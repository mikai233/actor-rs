use async_trait::async_trait;
use tracing::trace;

use actor_core::{Actor, CodecMessage, DynMessage};
use actor_core::actor::address::Address;
use actor_core::actor::context::{ActorContext1, ActorContext};
use actor_core::actor::props::Props;
use actor_core::actor_path::{ActorPath, TActorPath};
use actor_core::actor_path::root_actor_path::RootActorPath;
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;

use crate::cluster::Cluster;
use crate::heartbeat::cluster_heartbeat_receiver::cluster_event::ClusterEventWrap;
use crate::member::Member;

pub(crate) mod heartbeat;
mod cluster_event;

#[derive(Debug)]
pub(crate) struct ClusterHeartbeatReceiver {
    self_member: Option<Member>,
}

#[async_trait]
impl Actor for ClusterHeartbeatReceiver {
    async fn started(&mut self, context: &mut ActorContext1) -> anyhow::Result<()> {
        trace!("started {}", context.myself());
        Cluster::get(context.system()).subscribe(
            context.myself().clone(),
            |event| { ClusterEventWrap(event).into_dyn() },
        )?;
        Ok(())
    }

    async fn on_recv(&mut self, context: &mut ActorContext1, message: DynMessage) -> anyhow::Result<()> {
        Self::handle_message(self, context, message).await
    }
}

impl ClusterHeartbeatReceiver {
    pub(crate) fn new() -> Self {
        Self {
            self_member: None,
        }
    }

    pub(crate) fn props() -> Props {
        Props::new(|| Ok(Self::new()))
    }

    pub(crate) fn name() -> &'static str {
        "heartbeat_receiver"
    }

    pub(crate) fn path(address: Address) -> ActorPath {
        RootActorPath::new(address, "/").descendant(vec!["system", "cluster", Self::name()]).into()
    }
}
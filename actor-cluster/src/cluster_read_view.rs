use crate::cluster::Cluster;
use crate::cluster_event::{ClusterDomainEvent, CurrentClusterState, CurrentInternalStats};
use crate::member::Member;
use crate::reachability::Reachability;
use actor_core::actor::address::Address;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::{Actor, DynMessage};
use arc_swap::ArcSwap;
use async_trait::async_trait;
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub(crate) struct State {
    pub(crate) cluster_state: CurrentClusterState,
    pub(crate) reachability: Reachability,
    pub(crate) self_member: Member,
    pub(crate) latest_stats: CurrentInternalStats,
}

#[derive(Debug)]
pub(crate) struct ClusterReadView {
    pub(crate) cluster: Cluster,
    state: ArcSwap<State>,
    pub(crate) self_address: Address,

}

#[derive(Debug)]
struct EventBusListener;

impl EventBusListener {
    fn self_removed(&self) {}
}

#[async_trait]
impl Actor for EventBusListener {
    async fn started(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        let cluster = Cluster::get(context.system());
        cluster.subscribe::<dyn ClusterDomainEvent>(context.myself().clone());
        todo!()
    }

    async fn on_recv(&mut self, context: &mut ActorContext, message: DynMessage) -> anyhow::Result<()> {
        Self::handle_message(self, context, message).await
    }
}
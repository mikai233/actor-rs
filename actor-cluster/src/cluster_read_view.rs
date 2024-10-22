use crate::cluster::Cluster;
use crate::cluster_event::{ClusterDomainEvent, CurrentClusterState, CurrentInternalStats};
use crate::member::Member;
use crate::reachability::Reachability;
use actor_core::actor::address::Address;
use actor_core::actor::context::Context;
use actor_core::actor::Actor;
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use arc_swap::ArcSwap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl Actor for EventBusListener {
    type Context = Context;

    fn started(&mut self, ctx: &mut Self::Context) -> anyhow::Result<()> {
        let cluster = Cluster::get(ctx.system());
        cluster.subscribe::<dyn ClusterDomainEvent>(ctx.myself().clone());
        Ok(())
    }

    fn receive(&self) -> actor_core::actor::receive::Receive<Self> {
        todo!()
    }
}

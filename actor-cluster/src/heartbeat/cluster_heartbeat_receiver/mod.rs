use actor_core::actor::Actor;
use tracing::trace;

use actor_core::actor::address::Address;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::props::Props;
use actor_core::actor_path::root_actor_path::RootActorPath;
use actor_core::actor_path::{ActorPath, TActorPath};
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;

use crate::cluster::Cluster;
use crate::heartbeat::cluster_heartbeat_receiver::cluster_event::ClusterEventWrap;
use crate::member::Member;

mod cluster_event;
pub(crate) mod heartbeat;

#[derive(Debug)]
pub(crate) struct ClusterHeartbeatReceiver {
    self_member: Option<Member>,
}

impl Actor for ClusterHeartbeatReceiver {
    type Context = Context;

    fn started(&mut self, ctx: &mut Self::Context) -> anyhow::Result<()> {
        trace!("started {}", ctx.myself());
        Cluster::get(ctx.system()).subscribe(ctx.myself().clone(), |event| {
            ClusterEventWrap(event).into_dyn()
        })?;
        Ok(())
    }

    fn receive(&self) -> actor_core::actor::receive::Receive<Self> {
        todo!()
    }
}

impl ClusterHeartbeatReceiver {
    pub(crate) fn new() -> Self {
        Self { self_member: None }
    }

    pub(crate) fn props() -> Props {
        Props::new(|| Ok(Self::new()))
    }

    pub(crate) fn name() -> &'static str {
        "heartbeat_receiver"
    }

    pub(crate) fn path(address: Address) -> ActorPath {
        RootActorPath::new(address, "/")
            .descendant(vec!["system", "cluster", Self::name()])
            .into()
    }
}

use actor_core::actor::context::Context;
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;

use crate::cluster::Cluster;
use crate::member::{Member, MemberStatus};
use crate::on_member_status_changed_listener::cluster_event::ClusterEventWrap;

pub(crate) mod add_status_callback;
mod cluster_event;

pub(crate) struct OnMemberStatusChangedListener {
    cluster: Cluster,
    callback: Option<Box<dyn FnOnce() + Send>>,
    status: MemberStatus,
}

impl OnMemberStatusChangedListener {
    pub(crate) fn new(context: &mut Context, status: MemberStatus) -> Self {
        let cluster = Cluster::get(context.system());
        Self {
            callback: None,
            status,
            cluster,
        }
    }

    fn done(&mut self, context: &mut Context) {
        self.callback.take().into_foreach(|cb| cb());
        context.stop(context.myself());
    }

    fn is_triggered(&self, m: &Member) -> bool {
        &m.unique_address == self.cluster.self_unique_address() && m.status == self.status
    }
}

impl Actor for OnMemberStatusChangedListener {
    type Context = Context;

    fn started(&mut self, ctx: &mut Self::Context) -> anyhow::Result<()> {
        self.cluster.subscribe(ctx.myself().clone(), |event| {
            ClusterEventWrap(event).into_dyn()
        })?;
        debug_assert!(
            matches!(self.status, MemberStatus::Up) || matches!(self.status, MemberStatus::Removed)
        );
        Ok(())
    }

    fn stopped(&mut self, ctx: &mut Self::Context) -> anyhow::Result<()> {
        if matches!(self.status, MemberStatus::Removed) {
            self.callback.take().into_foreach(|cb| cb());
        };
        self.cluster.unsubscribe_cluster_event(ctx.myself())?;
        Ok(())
    }

    fn receive(&self) -> Receive<Self> {
        todo!()
    }
}

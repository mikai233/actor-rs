use async_trait::async_trait;

use actor_core::{Actor, CodecMessage, DynMessage};
use actor_core::actor::context::{Context, ActorContext};
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::ext::option_ext::OptionExt;

use crate::cluster::Cluster;
use crate::member::{Member, MemberStatus};
use crate::on_member_status_changed_listener::cluster_event::ClusterEventWrap;

mod cluster_event;
pub(crate) mod add_status_callback;

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

#[async_trait]
impl Actor for OnMemberStatusChangedListener {
    async fn started(&mut self, context: &mut Context) -> anyhow::Result<()> {
        self.cluster.subscribe(context.myself().clone(), |event| { ClusterEventWrap(event).into_dyn() })?;
        debug_assert!(matches!(self.status,MemberStatus::Up) || matches!(self.status,MemberStatus::Removed));
        Ok(())
    }

    async fn stopped(&mut self, context: &mut Context) -> anyhow::Result<()> {
        if matches!(self.status,MemberStatus::Removed) {
            self.callback.take().into_foreach(|cb| cb());
        };
        self.cluster.unsubscribe_cluster_event(context.myself())?;
        Ok(())
    }

    async fn on_recv(&mut self, context: &mut Context, message: DynMessage) -> anyhow::Result<()> {
        Self::handle_message(self, context, message).await
    }
}
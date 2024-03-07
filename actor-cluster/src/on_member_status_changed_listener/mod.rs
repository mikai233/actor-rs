use async_trait::async_trait;

use actor_core::Actor;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::ext::message_ext::UserMessageExt;
use actor_core::ext::option_ext::OptionExt;

use crate::cluster::Cluster;
use crate::member::MemberStatus;
use crate::on_member_status_changed_listener::cluster_event_wrap::ClusterEventWrap;

mod cluster_event_wrap;
pub(crate) mod add_status_callback;

pub(crate) struct OnMemberStatusChangedListener {
    callback: Option<Box<dyn FnOnce() + Send>>,
    status: MemberStatus,
}

impl OnMemberStatusChangedListener {
    pub(crate) fn new(status: MemberStatus) -> Self {
        Self {
            callback: None,
            status,
        }
    }
}

#[async_trait]
impl Actor for OnMemberStatusChangedListener {
    async fn started(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        let cluster = Cluster::get(context.system());
        cluster.subscribe_cluster_event(context.myself().clone(), |event| { ClusterEventWrap(event).into_dyn() });
        debug_assert!(matches!(self.status,MemberStatus::Up) || matches!(self.status,MemberStatus::Removed));
        Ok(())
    }

    async fn stopped(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        if matches!(self.status,MemberStatus::Removed) {
            self.callback.take().into_foreach(|cb| cb());
        };
        let cluster = Cluster::get(context.system());
        cluster.unsubscribe_cluster_event(context.myself());
        Ok(())
    }
}
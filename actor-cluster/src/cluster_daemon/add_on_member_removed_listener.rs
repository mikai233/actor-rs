use async_trait::async_trait;
use tracing::trace;

use actor_core::actor::actor_ref::ActorRefExt;
use actor_core::actor::actor_ref_factory::ActorRefFactory;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::props::Props;
use actor_core::Message;
use actor_derive::EmptyCodec;

use crate::cluster_daemon::ClusterDaemon;
use crate::member::MemberStatus;
use crate::on_member_status_changed_listener::{AddStatusCallback, OnMemberStatusChangedListener};

#[derive(EmptyCodec)]
pub(crate) struct AddOnMemberRemovedListener(pub(crate) Box<dyn FnOnce() + Send>);

#[async_trait]
impl Message for AddOnMemberRemovedListener {
    type A = ClusterDaemon;

    async fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
        let listener = context.spawn_anonymous(Props::new_with_ctx(|context| {
            Ok(OnMemberStatusChangedListener::new(context, MemberStatus::Removed))
        }))?;
        listener.cast_ns(AddStatusCallback(self.0));
        trace!("{} add callback on member removed", context.myself());
        Ok(())
    }
}
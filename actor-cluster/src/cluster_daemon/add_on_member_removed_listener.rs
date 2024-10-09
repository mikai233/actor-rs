use async_trait::async_trait;
use tracing::trace;

use actor_core::actor::context::{Context, ActorContext};
use actor_core::actor::props::Props;
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::actor_ref::ActorRefExt;
use actor_core::EmptyCodec;
use actor_core::Message;

use crate::cluster_daemon::ClusterDaemon;
use crate::member::MemberStatus;
use crate::on_member_status_changed_listener::add_status_callback::AddStatusCallback;
use crate::on_member_status_changed_listener::OnMemberStatusChangedListener;

#[derive(EmptyCodec)]
pub(crate) struct AddOnMemberRemovedListener(pub(crate) Box<dyn FnOnce() + Send>);

#[async_trait]
impl Message for AddOnMemberRemovedListener {
    type A = ClusterDaemon;

    async fn handle(self: Box<Self>, context: &mut Context, _actor: &mut Self::A) -> anyhow::Result<()> {
        let listener = context.spawn_anonymous(Props::new_with_ctx(|ctx| {
            Ok(OnMemberStatusChangedListener::new(ctx, MemberStatus::Removed))
        }))?;
        listener.cast_ns(AddStatusCallback(self.0));
        trace!("{} add callback on member removed", context.myself());
        Ok(())
    }
}
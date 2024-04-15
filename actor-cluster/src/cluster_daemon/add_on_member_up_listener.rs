use async_trait::async_trait;
use tracing::trace;

use actor_core::actor::context::{ActorContext, Context};
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
pub(crate) struct AddOnMemberUpListener(pub(crate) Box<dyn FnOnce() + Send>);

#[async_trait]
impl Message for AddOnMemberUpListener {
    type A = ClusterDaemon;

    async fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut Self::A) -> eyre::Result<()> {
        let listener = context.spawn_anonymous(Props::new_with_ctx(|ctx| {
            Ok(OnMemberStatusChangedListener::new(ctx, MemberStatus::Up))
        }))?;
        listener.cast_ns(AddStatusCallback(self.0));
        trace!("{} add callback on member up", context.myself());
        Ok(())
    }
}
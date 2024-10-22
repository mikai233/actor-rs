use std::fmt::Debug;

use actor_core::actor::behavior::Behavior;
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::message::handler::MessageHandler;
use actor_core::Message;
use tracing::trace;

use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::props::Props;
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::actor_ref::{ActorRef, ActorRefExt};

use crate::cluster_daemon::ClusterDaemon;
use crate::member::MemberStatus;
use crate::on_member_status_changed_listener::add_status_callback::AddStatusCallback;
use crate::on_member_status_changed_listener::OnMemberStatusChangedListener;

#[derive(Message, derive_more::Display)]
#[display("AddOnMemberRemovedListener")]
pub(crate) struct AddOnMemberRemovedListener(pub(crate) Box<dyn FnOnce() + Send>);

impl Debug for AddOnMemberRemovedListener {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AddOnMemberRemovedListener").finish()
    }
}

impl MessageHandler<ClusterDaemon> for AddOnMemberRemovedListener {
    fn handle(
        actor: &mut ClusterDaemon,
        ctx: &mut <ClusterDaemon as Actor>::Context,
        message: Self,
        sender: Option<ActorRef>,
        _: &Receive<ClusterDaemon>,
    ) -> anyhow::Result<Behavior<ClusterDaemon>> {
     let listener = context.spawn_anonymous(Props::new_with_ctx(|ctx| {
            Ok(OnMemberStatusChangedListener::new(
                ctx,
                MemberStatus::Removed,
            ))
        }))?;
        listener.cast_ns(AddStatusCallback(self.0));
        trace!("{} add callback on member removed", context.myself());
    Ok(Behavior::same())
    }
}

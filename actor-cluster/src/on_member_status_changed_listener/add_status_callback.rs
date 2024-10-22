use actor_core::{
    actor::{behavior::Behavior, receive::Receive, Actor},
    actor_ref::ActorRef,
    message::handler::MessageHandler,
    Message,
};
use std::fmt::Debug;

use crate::on_member_status_changed_listener::OnMemberStatusChangedListener;

#[derive(Message, derive_more::Display)]
pub(crate) struct AddStatusCallback(pub(crate) Box<dyn FnOnce() + Send>);

impl Debug for AddStatusCallback {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("AddStatusCallback").finish()
    }
}

impl MessageHandler<OnMemberStatusChangedListener> for AddStatusCallback {
    fn handle(
        actor: &mut OnMemberStatusChangedListener,
        ctx: &mut <OnMemberStatusChangedListener as Actor>::Context,
        message: Self,
        sender: Option<ActorRef>,
        _: &Receive<OnMemberStatusChangedListener>,
    ) -> anyhow::Result<Behavior<OnMemberStatusChangedListener>> {
        actor.callback = Some(message.0);
        Ok(Behavior::same())
    }
}

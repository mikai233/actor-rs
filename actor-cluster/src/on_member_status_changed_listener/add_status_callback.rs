use async_trait::async_trait;

use actor_core::actor::context::ActorContext;
use actor_core::Message;
use actor_derive::EmptyCodec;

use crate::on_member_status_changed_listener::OnMemberStatusChangedListener;

#[derive(EmptyCodec)]
pub(crate) struct AddStatusCallback(pub(crate) Box<dyn FnOnce() + Send>);

#[async_trait]
impl Message for AddStatusCallback {
    type A = OnMemberStatusChangedListener;

    async fn handle(self: Box<Self>, _context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        actor.callback = Some(self.0);
        Ok(())
    }
}
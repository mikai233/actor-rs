use async_trait::async_trait;

use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::props::Props;
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::ext::option_ext::OptionExt;
use actor_core::Message;
use actor_derive::EmptyCodec;

use crate::cluster_daemon::ClusterDaemon;
use crate::coordinated_shutdown_leave::CoordinatedShutdownLeave;

#[derive(Debug, EmptyCodec)]
pub(super) struct LeaveReq;

#[async_trait]
impl Message for LeaveReq {
    type A = ClusterDaemon;

    async fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut Self::A) -> eyre::Result<()> {
        let reply_to = context.sender().into_result()?.clone();
        context.spawn_anonymous(Props::new_with_ctx(|ctx| {
            Ok(CoordinatedShutdownLeave::new(ctx, reply_to))
        }))?;
        Ok(())
    }
}
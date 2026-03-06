use async_trait::async_trait;
use tracing::debug;

use actor_core::EmptyCodec;
use actor_core::Message;
use actor_core::actor::address::Address;
use actor_core::actor::context::ActorContext;

use crate::remote_watcher::RemoteWatcher;

#[derive(Debug, EmptyCodec)]
pub(super) struct ExpectedFirstHeartbeat {
    pub(super) from: Address,
}

#[async_trait]
impl Message for ExpectedFirstHeartbeat {
    type A = RemoteWatcher;

    async fn handle(
        self: Box<Self>,
        _context: &mut ActorContext,
        actor: &mut Self::A,
    ) -> anyhow::Result<()> {
        let address = self.from;
        if actor.watchee_by_nodes.contains_key(&address)
            && !actor.failure_detector.is_monitoring(&address)
        {
            debug!("Trigger extra expected heartbeat from [{}]", address);
            actor.failure_detector.heartbeat(address);
        }
        Ok(())
    }
}

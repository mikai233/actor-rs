use actor_core::actor::behavior::Behavior;
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::actor_ref::ActorRef;
use actor_core::message::handler::MessageHandler;
use actor_core::Message;
use tracing::debug;

use actor_core::actor::address::Address;

use crate::remote_watcher::RemoteWatcher;

#[derive(Debug, Message, derive_more::Display)]
#[display("ExpectedFirstHeartbeat {{ from: {from} }}")]
pub(super) struct ExpectedFirstHeartbeat {
    pub(super) from: Address,
}

impl MessageHandler<RemoteWatcher> for ExpectedFirstHeartbeat {
    fn handle(
        actor: &mut RemoteWatcher,
        _: &mut <RemoteWatcher as Actor>::Context,
        message: Self,
        _: Option<ActorRef>,
        _: &Receive<RemoteWatcher>,
    ) -> anyhow::Result<Behavior<RemoteWatcher>> {
        let address = message.from;
        if actor.watchee_by_nodes.contains_key(&address)
            && !actor.failure_detector.is_monitoring(&address)
        {
            debug!("Trigger extra expected heartbeat from [{}]", address);
            actor.failure_detector.heartbeat(address);
        }
        Ok(Behavior::same())
    }
}

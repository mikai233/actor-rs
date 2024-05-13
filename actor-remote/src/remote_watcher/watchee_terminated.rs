use async_trait::async_trait;
use tracing::debug;

use actor_core::{CodecMessage, DynMessage, Message};
use actor_core::actor::context::ActorContext;
use actor_core::actor_ref::{ActorRef, ActorRefSystemExt};
use actor_core::EmptyCodec;
use actor_core::message::death_watch_notification::DeathWatchNotification;
use actor_core::message::terminated::Terminated;

use crate::remote_watcher::RemoteWatcher;

#[derive(Debug, EmptyCodec)]
pub(super) struct WatcheeTerminated(pub(super) Terminated);

impl WatcheeTerminated {
    pub(super) fn new(terminated: Terminated) -> DynMessage {
        Self(terminated).into_dyn()
    }
}

#[async_trait]
impl Message for WatcheeTerminated {
    type A = RemoteWatcher;

    async fn handle(self: Box<Self>, _context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let Terminated { actor: watchee, existence_confirmed, address_terminated } = self.0;
        debug!("Watchee terminated: [{}]", watchee.path());
        // When watchee is stopped it sends DeathWatchNotification to this RemoteWatcher,
        // which will propagate it to all watchers of this watchee.
        // address_terminated case is already handled by the watcher itself in DeathWatch trait
        if !address_terminated {
            if let Some(watchers) = actor.watching.get(&watchee) {
                let notify = DeathWatchNotification {
                    actor: watchee.clone(),
                    existence_confirmed,
                    address_terminated,
                };
                for watcher in watchers {
                    watcher.cast_system(notify.clone(), ActorRef::no_sender());
                }
            }
        }
        actor.remove_watchee(&watchee);
        Ok(())
    }
}
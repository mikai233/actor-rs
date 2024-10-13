use crate::actor::actor_system::ActorSystem;
use crate::actor::mailbox::Mailbox;
use crate::actor::props::Props;
use crate::actor_ref::local_ref::SignalReceiver;
use crate::actor_ref::ActorRef;
use crate::provider::TActorRefProvider;

#[derive(Debug, derive_more::Constructor)]
pub struct Provider<P: TActorRefProvider> {
    pub(crate) provider: P,
    pub(crate) spawns: Vec<ActorSpawn>,
}

#[derive(Debug, derive_more::Constructor)]
pub struct ActorSpawn {
    pub(crate) props: Props,
    pub(crate) myself: ActorRef,
    pub(crate) signal_rx: SignalReceiver,
    pub(crate) mailbox: Mailbox,
}

impl ActorSpawn {
    pub(crate) fn spawn(self, system: ActorSystem) -> anyhow::Result<()> {
        let Self {
            props,
            myself,
            signal_rx,
            mailbox,
        } = self;
        props.spawn(myself, signal_rx, mailbox, system)
    }
}

use actor_core::{
    actor::actor_selection::ActorSelectionMessage,
    message::{
        address_terminated::AddressTerminated,
        death_watch_notification::DeathWatchNotification,
        identify::{ActorIdentity, Identify},
        poison_pill::PoisonPill,
        resume::Resume,
        suspend::Suspend,
        terminate::Terminate,
        unwatch::Unwatch,
        watch::Watch,
    },
};
use codec::{register_message, MessageCodecRegistry};
use remote_watcher::{
    artery_heartbeat::ArteryHeartbeat, artery_heartbeat_rsp::ArteryHeartbeatRsp,
    heartbeat::Heartbeat, heartbeat_rsp::HeartbeatRsp,
};

pub const REFERENCE: &'static str = include_str!("../reference.json");

pub mod artery;
pub mod codec;
pub mod config;
mod failure_detector;
pub mod remote_actor_ref;
pub mod remote_provider;
mod remote_watcher;

pub fn register_remote_system_message(registry: &mut dyn MessageCodecRegistry) {
    register_message::<AddressTerminated>(registry);
    register_message::<DeathWatchNotification>(registry);
    register_message::<Identify>(registry);
    register_message::<ActorIdentity>(registry);
    register_message::<PoisonPill>(registry);
    register_message::<Resume>(registry);
    register_message::<Suspend>(registry);
    register_message::<Terminate>(registry);
    register_message::<Unwatch>(registry);
    register_message::<Watch>(registry);
    register_message::<ActorSelectionMessage>(registry);
    register_message::<ArteryHeartbeat>(registry);
    register_message::<ArteryHeartbeatRsp>(registry);
    register_message::<Heartbeat>(registry);
    register_message::<HeartbeatRsp>(registry);
}

#[cfg(test)]
mod test {
    use tracing::Level;

    use actor_core::ext::init_logger;

    #[ctor::ctor]
    fn init() {
        init_logger(Level::DEBUG)
    }
}

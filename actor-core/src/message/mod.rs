use dyn_clone::DynClone;

use crate::DynMessage;
use crate::message::message_registry::MessageRegistry;

pub mod death_watch_notification;
pub mod terminated;
pub mod terminate;
pub mod watch;
pub mod unwatch;
pub mod poison_pill;
pub mod execute;
pub mod suspend;
pub mod resume;
pub mod failed;
pub mod identify;
pub mod message_registry;
pub mod stop_child;
pub mod message_buffer;
pub mod address_terminated;
pub(crate) mod task_finish;

pub trait MessageDecoder: Send + Sync + DynClone + 'static {
    fn decode(&self, bytes: &[u8], reg: &MessageRegistry) -> anyhow::Result<DynMessage>;
}

dyn_clone::clone_trait_object!(MessageDecoder);


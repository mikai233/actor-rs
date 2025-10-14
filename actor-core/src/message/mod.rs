use dyn_clone::DynClone;

use crate::DynMessage;
use crate::message::message_registry::MessageRegistry;

pub mod address_terminated;
pub mod death_watch_notification;
pub mod execute;
pub mod failed;
pub mod identify;
pub mod message_buffer;
pub mod message_registry;
pub mod poison_pill;
pub mod resume;
pub mod stop_child;
pub mod suspend;
pub(crate) mod task_finish;
pub mod terminate;
pub mod terminated;
pub mod unwatch;
pub mod watch;

pub trait MessageDecoder: Send + Sync + DynClone + 'static {
    fn decode(&self, bytes: &[u8], reg: &MessageRegistry) -> anyhow::Result<DynMessage>;
}

dyn_clone::clone_trait_object!(MessageDecoder);

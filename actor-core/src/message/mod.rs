use bincode::error::DecodeError;
use dyn_clone::DynClone;

use crate::DynMessage;
use crate::message::message_registration::MessageRegistration;

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
pub mod message_registration;
pub mod stop_child;
pub mod message_buffer;

pub trait MessageDecoder: Send + Sync + DynClone + 'static {
    fn decode(&self, bytes: &[u8], reg: &MessageRegistration) -> Result<DynMessage, DecodeError>;
}

dyn_clone::clone_trait_object!(MessageDecoder);


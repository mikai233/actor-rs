use bincode::error::DecodeError;
use dyn_clone::DynClone;

use crate::DynMessage;
use crate::message::message_registration::MessageRegistration;

pub trait MessageDecoder: Send + Sync + DynClone + 'static {
    fn decode(&self, bytes: &[u8], reg: &MessageRegistration) -> Result<DynMessage, DecodeError>;
}

dyn_clone::clone_trait_object!(MessageDecoder);
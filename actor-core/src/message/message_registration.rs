use std::collections::HashMap;
use std::fmt::{Debug, Formatter};

use bincode::{Decode, Encode};
use bincode::error::{DecodeError, EncodeError};

use crate::{CodecMessage, DynMessage};
use crate::actor::actor_selection::ActorSelectionMessage;
use crate::actor::decoder::MessageDecoder;
use crate::message::death_watch_notification::DeathWatchNotification;
use crate::message::identify::{ActorIdentity, Identify};
use crate::message::terminate::Terminate;
use crate::message::unwatch::Unwatch;
use crate::message::watch::Watch;

#[derive(Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub struct IDPacket {
    id: u32,
    bytes: Vec<u8>,
}

#[derive(Clone)]
pub struct MessageRegistration {
    pub next_id: u32,
    pub name_id: HashMap<&'static str, u32>,
    pub decoder: HashMap<u32, Box<dyn MessageDecoder>>,
}

impl Debug for MessageRegistration {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_struct("MessageRegistration")
            .field("next_id", &self.next_id)
            .field("name_id", &self.name_id)
            .field("decoder", &"..")
            .finish()
    }
}

impl MessageRegistration {
    pub fn new() -> Self {
        let mut reg = Self {
            next_id: 0,
            name_id: HashMap::new(),
            decoder: HashMap::new(),
        };
        reg.register_all_system_message();
        reg
    }

    pub fn register<M>(&mut self) where M: CodecMessage {
        let name = std::any::type_name::<M>();
        let decoder = M::decoder().expect(&*format!("{} decoder is empty", name));
        assert!(!self.name_id.contains_key(name), "message {} already registered", name);
        self.name_id.insert(name, self.next_id);
        self.decoder.insert(self.next_id, decoder);
        self.next_id += 1;
    }

    pub fn encode_boxed(&self, message: &DynMessage) -> Result<IDPacket, EncodeError> {
        let DynMessage { name, boxed: boxed_message, .. } = message;
        self.encode(name, &**boxed_message)
    }

    pub fn encode(&self, name: &'static str, message: &dyn CodecMessage) -> Result<IDPacket, EncodeError> {
        let id = *self.name_id.get(name).ok_or(EncodeError::OtherString(format!("message {} not register", name)))?;
        let bytes = message.encode(self)?;
        let packet = IDPacket {
            id,
            bytes,
        };
        Ok(packet)
    }

    pub fn decode(&self, packet: IDPacket) -> Result<DynMessage, DecodeError> {
        let id = packet.id;
        let decoder = self.decoder.get(&id).ok_or(DecodeError::OtherString(format!("message {} not register", id)))?;
        let message = decoder.decode(&packet.bytes, self)?;
        Ok(message)
    }

    fn register_all_system_message(&mut self) {
        self.register::<DeathWatchNotification>();
        self.register::<Terminate>();
        self.register::<Watch>();
        self.register::<Unwatch>();
        self.register::<ActorSelectionMessage>();
        self.register::<Identify>();
        self.register::<ActorIdentity>();
    }
}
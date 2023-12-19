use std::collections::HashMap;
use std::fmt::{Debug, Formatter};

use anyhow::anyhow;
use bincode::{Decode, Encode};

use actor_core::{CodecMessage, DynMessage};
use actor_core::actor::decoder::MessageDecoder;
use actor_core::message::death_watch_notification::DeathWatchNotification;
use actor_core::message::terminate::Terminate;
use actor_core::message::unwatch::Unwatch;
use actor_core::message::watch::Watch;

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

    pub(crate) fn encode_boxed(&self, message: DynMessage) -> anyhow::Result<IDPacket> {
        let DynMessage { name, boxed: boxed_message, .. } = message;
        self.encode(name, &*boxed_message)
    }

    pub(crate) fn encode(&self, name: &'static str, message: &dyn CodecMessage) -> anyhow::Result<IDPacket> {
        let id = *self.name_id.get(name).ok_or(anyhow!("message {} not register", name))?;
        let bytes = message.encode()?;
        let packet = IDPacket {
            id,
            bytes,
        };
        Ok(packet)
    }

    pub(crate) fn decode(&self, packet: IDPacket) -> anyhow::Result<DynMessage> {
        let id = packet.id;
        let decoder = self.decoder.get(&id).ok_or(anyhow!("message {} not register", id))?;
        let message = decoder.decode(&packet.bytes)?;
        Ok(message)
    }

    fn register_all_system_message(&mut self) {
        self.register::<DeathWatchNotification>();
        self.register::<Terminate>();
        self.register::<Watch>();
        self.register::<Unwatch>();
    }
}
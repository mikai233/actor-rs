use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use crate::actor::{BoxedMessage, DynamicMessage, Message, SystemMessage};
use crate::decoder::MessageDecoder;
use crate::ext::{decode_bytes, encode_bytes};

pub(crate) mod death_watch_notification;
pub(crate) mod terminated;
pub(crate) mod terminate;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct IDPacket {
    id: u32,
    bytes: Vec<u8>,
}

pub struct MessageRegistration {
    pub id: u32,
    pub name_id: HashMap<&'static str, u32>,
    pub decoder: HashMap<u32, Box<dyn MessageDecoder>>,
}

impl Debug for MessageRegistration {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MessageRegistration")
            .field("id", &self.id)
            .field("name_id", &self.name_id)
            .field("decoder", &"..")
            .finish()
    }
}

impl MessageRegistration {
    pub fn new() -> Self {
        let mut reg = Self {
            id: 0,
            name_id: HashMap::new(),
            decoder: HashMap::new(),
        };
        reg.register_all_system_message();
        reg
    }

    pub fn register<M>(&mut self) where M: Message {
        let name = std::any::type_name::<M>();
        let decoder = M::decoder().expect(&*format!("{} decoder is empty", name));
        self.name_id.insert(name, self.id);
        self.decoder.insert(self.id, decoder);
        self.id += 1;
    }

    pub(crate) fn register_system<M>(&mut self) where M: SystemMessage {
        let name = std::any::type_name::<M>();
        let decoder = M::decoder().expect(&*format!("{} decoder is empty", name));
        self.name_id.insert(name, self.id);
        self.decoder.insert(self.id, decoder);
        self.id += 1;
    }

    pub(crate) fn encode(&self, boxed_message: BoxedMessage) -> anyhow::Result<IDPacket> {
        let BoxedMessage { name, inner: message } = boxed_message;
        let id = *self.name_id.get(name).ok_or(anyhow!("message {} not register", name))?;
        let bytes = message.encode().ok_or(anyhow!("{} encoder is empty", name))??;
        let packet = IDPacket {
            id,
            bytes,
        };
        Ok(packet)
    }

    pub(crate) fn decode(&self, packet: IDPacket) -> anyhow::Result<DynamicMessage> {
        let id = packet.id;
        let decoder = self.decoder.get(&id).ok_or(anyhow!("message {} not register", id))?;
        decoder.decode(&packet.bytes)
    }

    fn register_all_system_message(&mut self) {}
}
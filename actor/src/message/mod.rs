use std::collections::HashMap;
use std::fmt::{Debug, Formatter};

use anyhow::anyhow;
use serde::{Deserialize, Serialize};

use crate::{CodecMessage, DynMessage};
use crate::decoder::MessageDecoder;
use crate::message::death_watch_notification::DeathWatchNotification;
use crate::message::terminate::Terminate;
use crate::message::unwatch::Unwatch;
use crate::message::watch::Watch;
use crate::provider::ActorRefProvider;

pub(crate) mod death_watch_notification;
pub(crate) mod terminated;
pub(crate) mod terminate;
pub(crate) mod watch;
pub(crate) mod unwatch;
pub mod poison_pill;

#[derive(Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub(crate) struct IDPacket {
    id: u32,
    bytes: Vec<u8>,
}

#[derive(Clone)]
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

    pub fn register<M>(&mut self) where M: CodecMessage {
        let name = std::any::type_name::<M>();
        let decoder = M::decoder().expect(&*format!("{} decoder is empty", name));
        assert!(!self.name_id.contains_key(name), "message {} already registered", name);
        self.name_id.insert(name, self.id);
        self.decoder.insert(self.id, decoder);
        self.id += 1;
    }

    pub(crate) fn encode_boxed(&self, message: DynMessage) -> anyhow::Result<IDPacket> {
        let DynMessage { name, boxed: boxed_message, .. } = message;
        self.encode(name, &*boxed_message)
    }

    pub(crate) fn encode(&self, name: &'static str, message: &dyn CodecMessage) -> anyhow::Result<IDPacket> {
        let id = *self.name_id.get(name).ok_or(anyhow!("message {} not register", name))?;
        let bytes = message.encode().ok_or(anyhow!("{} encoder is empty", name))??;
        let packet = IDPacket {
            id,
            bytes,
        };
        Ok(packet)
    }

    pub(crate) fn decode(&self, provider: &ActorRefProvider, packet: IDPacket) -> anyhow::Result<DynMessage> {
        let id = packet.id;
        let decoder = self.decoder.get(&id).ok_or(anyhow!("message {} not register", id))?;
        decoder.decode(provider, &packet.bytes)
    }

    fn register_all_system_message(&mut self) {
        self.register::<DeathWatchNotification>();
        self.register::<Terminate>();
        self.register::<Watch>();
        self.register::<Unwatch>();
    }
}
use std::any::type_name;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};

use bincode::{Decode, Encode};
use eyre::eyre;

use crate::{CodecMessage, DynMessage};
use crate::actor::actor_selection::ActorSelectionMessage;
use crate::message::address_terminated::AddressTerminated;
use crate::message::death_watch_notification::DeathWatchNotification;
use crate::message::identify::{ActorIdentity, Identify};
use crate::message::MessageDecoder;
use crate::message::poison_pill::PoisonPill;
use crate::message::resume::Resume;
use crate::message::suspend::Suspend;
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
    pub next_user_id: u32,
    pub next_system_id: u32,
    pub name_id: HashMap<&'static str, u32>,
    pub decoder: HashMap<u32, Box<dyn MessageDecoder>>,
}

impl Debug for MessageRegistration {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_struct("MessageRegistration")
            .field("next_user_id", &self.next_user_id)
            .field("next_system_id", &self.next_system_id)
            .field("name_id", &self.name_id)
            .field("decoder", &"..")
            .finish()
    }
}

impl MessageRegistration {
    pub fn new() -> Self {
        let mut reg = Self {
            next_user_id: 500,
            next_system_id: 0,
            name_id: HashMap::new(),
            decoder: HashMap::new(),
        };
        reg.register_all_system_message();
        reg
    }

    fn register<M>(&mut self, id: u32) where M: CodecMessage {
        let name = type_name::<M>();
        let decoder = M::decoder().expect(&*format!("{} decoder is empty", name));
        assert!(!self.name_id.contains_key(name), "message {} already registered", name);
        self.name_id.insert(name, id);
        self.decoder.insert(id, decoder);
    }

    pub fn register_user<M>(&mut self) where M: CodecMessage {
        self.register::<M>(self.next_user_id);
        self.next_user_id += 1;
    }

    pub fn register_system<M>(&mut self) where M: CodecMessage {
        self.register::<M>(self.next_system_id);
        self.next_system_id += 1;
    }


    pub fn encode_boxed(&self, message: &DynMessage) -> eyre::Result<IDPacket> {
        let DynMessage { name, message, .. } = message;
        self.encode(name, &**message)
    }

    pub fn encode(&self, name: &'static str, message: &dyn CodecMessage) -> eyre::Result<IDPacket> {
        let id = *self.name_id.get(name).ok_or(eyre!("message {} is not registered", name))?;
        let bytes = message.encode(self)?;
        let packet = IDPacket {
            id,
            bytes,
        };
        Ok(packet)
    }

    pub fn decode(&self, packet: IDPacket) -> eyre::Result<DynMessage> {
        let id = packet.id;
        let decoder = self.decoder.get(&id).ok_or(eyre!("message with id {} is not registered", id))?;
        let message = decoder.decode(&packet.bytes, self)?;
        Ok(message)
    }

    fn register_all_system_message(&mut self) {
        self.register_system::<AddressTerminated>();
        self.register_system::<DeathWatchNotification>();
        self.register_system::<Identify>();
        self.register_system::<ActorIdentity>();
        self.register_system::<PoisonPill>();
        self.register_system::<Resume>();
        self.register_system::<Suspend>();
        self.register_system::<Terminate>();
        self.register_system::<Unwatch>();
        self.register_system::<Watch>();
        self.register_system::<ActorSelectionMessage>();
    }
}
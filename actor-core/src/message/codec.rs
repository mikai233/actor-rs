use std::any::type_name;
use std::fmt::{Debug, Formatter};

use ahash::{HashMap, HashMapExt};
use anyhow::anyhow;
use bincode::{Decode, Encode};

use crate::actor::actor_selection::ActorSelectionMessage;
use crate::message::address_terminated::AddressTerminated;
use crate::message::death_watch_notification::DeathWatchNotification;
use crate::message::identify::{ActorIdentity, Identify};
use crate::message::poison_pill::PoisonPill;
use crate::message::resume::Resume;
use crate::message::suspend::Suspend;
use crate::message::terminate::Terminate;
use crate::message::unwatch::Unwatch;
use crate::message::watch::Watch;
use crate::message::Message;

#[derive(Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub struct IDPacket {
    id: u32,
    bytes: Vec<u8>,
}

#[derive(Clone)]
pub struct MessageRegistry {
    pub next_user: u32,
    pub next_system: u32,
    pub id: HashMap<&'static str, u32>,
    pub decoder: HashMap<u32, Box<dyn MessageDecoder>>,
}

impl Debug for MessageRegistry {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_struct("MessageRegistry")
            .field("next_user", &self.next_user)
            .field("next_system", &self.next_system)
            .field("id", &self.id)
            .field("decoder", &"..")
            .finish()
    }
}

impl MessageRegistry {
    pub fn new() -> Self {
        let mut reg = Self {
            next_user: 500,
            next_system: 0,
            id: HashMap::new(),
            decoder: HashMap::new(),
        };
        reg.register_all_system_message();
        reg
    }

    pub fn register<M>(&mut self, id: u32) where M: CodecMessage {
        let name = type_name::<M>();
        let decoder = M::decoder().expect(&*format!("{} decoder is empty", name));
        assert!(!self.id.contains_key(name), "message {} already registered", name);
        self.id.insert(name, id);
        self.decoder.insert(id, decoder);
    }

    pub fn register_user<M>(&mut self) where M: CodecMessage {
        self.register::<M>(self.next_user);
        self.next_user += 1;
    }

    pub fn register_system<M>(&mut self) where M: CodecMessage {
        self.register::<M>(self.next_system);
        self.next_system += 1;
    }

    pub fn encode_boxed(&self, message: DynMessage) -> anyhow::Result<IDPacket> {
        let DynMessage { name, message, .. } = message;
        self.encode(name, message)
    }

    pub fn encode(&self, name: &'static str, message: Box<dyn CodecMessage>) -> anyhow::Result<IDPacket> {
        let id = *self.id.get(name).ok_or(anyhow!("message {} is not registered", name))?;
        let bytes = message.encode(self)?;
        let packet = IDPacket {
            id,
            bytes,
        };
        Ok(packet)
    }

    pub fn decode(&self, packet: IDPacket) -> anyhow::Result<DynMessage> {
        let id = packet.id;
        let decoder = self.decoder.get(&id).ok_or(anyhow!("message with id {} is not registered", id))?;
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

impl Default for MessageRegistry {
    fn default() -> Self {
        MessageRegistry::new()
    }
}

pub type EncoderType = Box<dyn Fn(&dyn MessageCodec) -> anyhow::Result<Vec<u8>>>;

pub type DecoderType = Box<dyn Fn(Vec<u8>) -> anyhow::Result<Box<dyn MessageCodec>>>;

pub trait MessageCodec: Message {
    fn into_message(self: Box<Self>) -> Box<dyn Message>;

    fn encode(&self) -> anyhow::Result<Vec<u8>>;

    fn decode(bytes: Vec<u8>) -> anyhow::Result<Box<dyn MessageCodec>>
    where
        Self: Sized;
}

pub struct MessageCodecRegistry {
    pub encoder: HashMap<&'static str, EncoderType>,
    pub decoder: HashMap<&'static str, DecoderType>,
}

impl MessageCodecRegistry {
    pub fn new() -> Self {
        Self {
            encoder: HashMap::new(),
            decoder: HashMap::new(),
        }
    }

    fn register_encoder<M>(&mut self)
    where
        M: MessageCodec,
    {
        self.encoder
            .insert(M::signature_sized(), Box::new(|m| m.encode()));
    }

    fn register_decoder<M>(&mut self)
    where
        M: MessageCodec,
    {
        self.decoder
            .insert(M::signature_sized(), Box::new(|bytes| M::decode(bytes)));
    }

    pub fn register<M>(&mut self)
    where
        M: MessageCodec,
    {
        self.register_encoder::<M>();
        self.register_decoder::<M>();
    }

    pub fn encode<M>(&self, message: &M) -> anyhow::Result<Vec<u8>>
    where
        M: AsRef<dyn MessageCodec>,
    {
        let message = message.as_ref();
        let signature = message.signature();
        let encoder = self
            .encoder
            .get(signature)
            .ok_or_else(|| anyhow::anyhow!("Encoder {signature} not found"))?;
        encoder(message)
    }

    pub fn decode(&self, signature: &str, bytes: Vec<u8>) -> anyhow::Result<Box<dyn MessageCodec>> {
        let decoder = self
            .decoder
            .get(signature)
            .ok_or_else(|| anyhow::anyhow!("Decoder {signature} not found"))?;
        decoder(bytes)
    }
}
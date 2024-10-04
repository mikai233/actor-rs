use super::Message;

pub trait MessageCodec: Message {
    fn into_message(self: Box<Self>) -> Box<dyn Message>;

    fn encode(&self) -> anyhow::Result<Vec<u8>>;

    fn decode(bytes: &[u8]) -> anyhow::Result<Box<dyn MessageCodec>>
    where
        Self: Sized;
}

impl<T> MessageCodec for T
where
    T: Message + serde::Serialize + serde::de::DeserializeOwned + 'static,
{
    fn into_message(self: Box<Self>) -> Box<dyn Message> {
        self
    }

    fn encode(&self) -> anyhow::Result<Vec<u8>> {
        bincode::serialize(self).map_err(Into::into)
    }

    fn decode(bytes: &[u8]) -> anyhow::Result<Box<dyn MessageCodec>> {
        bincode::deserialize(bytes).map_err(Into::into)
    }
}

pub trait MessageCodecRegistry {
    fn encode(&self, message: &dyn MessageCodec) -> anyhow::Result<Vec<u8>>;

    fn decode(&self, bytes: &[u8]) -> anyhow::Result<Box<dyn MessageCodec>>;
}

// fn register_all_system_message(&mut self) {
//     self.register_system::<AddressTerminated>();
//     self.register_system::<DeathWatchNotification>();
//     self.register_system::<Identify>();
//     self.register_system::<ActorIdentity>();
//     self.register_system::<PoisonPill>();
//     self.register_system::<Resume>();
//     self.register_system::<Suspend>();
//     self.register_system::<Terminate>();
//     self.register_system::<Unwatch>();
//     self.register_system::<Watch>();
//     self.register_system::<ActorSelectionMessage>();
// }

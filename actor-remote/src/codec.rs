use actor_core::message::{DynMessage, Message, Signature};

pub type DecoderFn = Box<dyn Fn(&[u8], &dyn MessageCodecRegistry) -> anyhow::Result<DynMessage>>;

pub type EncoderFn = Box<dyn Fn(&dyn Message, &dyn MessageCodecRegistry) -> anyhow::Result<Vec<u8>>>;

pub trait MessageCodec: Sized {
    type M: Message;
    fn encode(message: &Self::M, registry: &dyn MessageCodecRegistry) -> anyhow::Result<Vec<u8>>;

    fn decode(bytes: &[u8], registry: &dyn MessageCodecRegistry) -> anyhow::Result<Self::M>;
}

pub trait MessageCodecRegistry: Send + Sync {
    fn encode(&self, message: &dyn Message) -> anyhow::Result<Vec<u8>>;

    fn decode(&self, bytes: &[u8]) -> anyhow::Result<DynMessage>;

    fn register_system(&mut self, signature: Signature, decoder: DecoderFn, encoder: EncoderFn);
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

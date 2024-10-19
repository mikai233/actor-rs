use std::fmt::Debug;

use crate::remote_watcher::{
    artery_heartbeat::ArteryHeartbeat, artery_heartbeat_rsp::ArteryHeartbeatRsp,
    heartbeat::Heartbeat, heartbeat_rsp::HeartbeatRsp,
};
use actor_core::actor::actor_selection::{ActorSelectionMessage, SelectionPathElement};
use actor_core::message::address_terminated::AddressTerminated;
use actor_core::message::identify::{ActorIdentity, Identify};
use actor_core::message::poison_pill::PoisonPill;
use actor_core::message::resume::Resume;
use actor_core::message::suspend::Suspend;
use actor_core::message::terminate::Terminate;
use actor_core::message::unwatch::Unwatch;
use actor_core::message::watch::Watch;
use actor_core::{
    actor::actor_system::ActorSystem,
    message::{
        death_watch_notification::DeathWatchNotification, downcast_ref, DynMessage, Message,
        Signature,
    },
};
use anyhow::anyhow;
use serde::{Deserialize, Serialize};

pub type DecoderFn =
Box<dyn Fn(&[u8], &ActorSystem, &dyn MessageCodecRegistry) -> anyhow::Result<DynMessage>>;

pub type EncoderFn =
Box<dyn Fn(&dyn Message, &ActorSystem, &dyn MessageCodecRegistry) -> anyhow::Result<Vec<u8>>>;

pub trait MessageCodec: Sized {
    type M: Message;

    fn encode(
        message: &Self::M,
        system: &ActorSystem,
        registry: &dyn MessageCodecRegistry,
    ) -> anyhow::Result<Vec<u8>>;

    fn decode(
        bytes: &[u8],
        system: &ActorSystem,
        registry: &dyn MessageCodecRegistry,
    ) -> anyhow::Result<Self::M>;
}

pub trait MessageCodecRegistry: Debug + Send + Sync {
    fn encode(&self, message: &dyn Message, system: &ActorSystem) -> anyhow::Result<Vec<u8>>;

    fn decode(&self, bytes: &[u8], system: &ActorSystem) -> anyhow::Result<DynMessage>;

    fn register_system(&mut self, signature: Signature, decoder: DecoderFn, encoder: EncoderFn);
}

pub fn register_system<M>(registry: &mut dyn MessageCodecRegistry)
where
    M: Message + MessageCodec,
{
    registry.register_system(
        M::signature_sized(),
        Box::new(|bytes, system, registry| {
            let message = M::decode(bytes, system, registry)?;
            Ok(Box::new(message))
        }),
        Box::new(|message, system, registry| {
            let message = downcast_ref(message).ok_or(anyhow!(
                "Downcast {} to {} failed",
                message.signature(),
                M::signature_sized()
            ))?;
            let bytes = M::encode(message, system, registry)?;
            Ok(bytes)
        }),
    );
}

macro_rules! impl_message_codec {
    ($message:ty) => {
        impl MessageCodec for $message {
            type M = $message;

            fn encode(
                message: &Self::M,
                _: &ActorSystem,
                _: &dyn MessageCodecRegistry,
            ) -> anyhow::Result<Vec<u8>> {
                let bytes = bincode::serialize(message)?;
                Ok(bytes)
            }

            fn decode(
                bytes: &[u8],
                _: &ActorSystem,
                _: &dyn MessageCodecRegistry,
            ) -> anyhow::Result<Self::M> {
                let message = bincode::deserialize(bytes)?;
                Ok(message)
            }
        }
    };
    ($($message:ty),*) => {
        $(impl_message_codec!($message);)*
    };
}

impl_message_codec!(
    AddressTerminated,
    DeathWatchNotification,
    Identify,
    ActorIdentity,
    PoisonPill,
    Resume,
    Suspend,
    Terminate,
    Unwatch,
    Watch
);

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ActorSelectionMessagePacket {
    message_bytes: Vec<u8>,
    elements_bytes: Vec<u8>,
    wildcard_fan_out: bool,
}

impl MessageCodec for ActorSelectionMessage {
    type M = ActorSelectionMessage;

    fn encode(
        message: &Self::M,
        system: &ActorSystem,
        registry: &dyn MessageCodecRegistry,
    ) -> anyhow::Result<Vec<u8>> {
        let message_bytes = registry.encode(&**message.message(), system)?;
        let elements_bytes = bincode::serialize(message.elements())?;
        let packet = ActorSelectionMessagePacket {
            message_bytes,
            elements_bytes,
            wildcard_fan_out: message.wildcard_fan_out(),
        };
        let bytes = bincode::serialize(&packet)?;
        Ok(bytes)
    }

    fn decode(
        bytes: &[u8],
        system: &ActorSystem,
        registry: &dyn MessageCodecRegistry,
    ) -> anyhow::Result<Self::M> {
        let packet: ActorSelectionMessagePacket = bincode::deserialize(bytes)?;
        let message = registry.decode(&packet.message_bytes, system)?;
        let elements: Vec<SelectionPathElement> = bincode::deserialize(&packet.elements_bytes)?;
        ActorSelectionMessage::new(message, elements, packet.wildcard_fan_out)
    }
}

pub fn register_remote_system_message(registry: &mut dyn MessageCodecRegistry) {
    register_system::<AddressTerminated>(registry);
    register_system::<DeathWatchNotification>(registry);
    register_system::<Identify>(registry);
    register_system::<ActorIdentity>(registry);
    register_system::<PoisonPill>(registry);
    register_system::<Resume>(registry);
    register_system::<Suspend>(registry);
    register_system::<Terminate>(registry);
    register_system::<Unwatch>(registry);
    register_system::<Watch>(registry);
    register_system::<ActorSelectionMessage>(registry);
    register_system::<ArteryHeartbeat>(registry);
    register_system::<ArteryHeartbeatRsp>(registry);
    register_system::<Heartbeat>(registry);
    register_system::<HeartbeatRsp>(registry);
}

use std::fmt::Debug;

use crate::remote_watcher::{
    artery_heartbeat::ArteryHeartbeat, artery_heartbeat_rsp::ArteryHeartbeatRsp,
    heartbeat::Heartbeat, heartbeat_rsp::HeartbeatRsp,
};
use actor_core::actor::actor_selection::{ActorSelectionMessage, SelectionPathElement};
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::actor_ref::PROVIDER;
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
use anyhow::{anyhow, Context};
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

    fn register(&mut self, signature: Signature, decoder: DecoderFn, encoder: EncoderFn);
}

pub fn register_message<M>(registry: &mut dyn MessageCodecRegistry)
where
    M: Message + MessageCodec,
{
    registry.register(
        M::signature_sized(),
        Box::new(|bytes, system, registry| {
            let provider = system.provider().clone();
            let message = PROVIDER.sync_scope(provider, || {
                M::decode(bytes, system, registry)
                    .with_context(|| anyhow!("Decode message `{}` failed", M::signature_sized()))
            })?;
            Ok(Box::new(message))
        }),
        Box::new(|message, system, registry| {
            let message = downcast_ref(message).ok_or(anyhow!(
                "Downcast message `{}` to `{}` failed",
                message.signature(),
                M::signature_sized()
            ))?;
            let bytes = M::encode(message, system, registry)
                .with_context(|| anyhow!("Encode message `{}` failed", M::signature_sized()))?;
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
    register_message::<AddressTerminated>(registry);
    register_message::<DeathWatchNotification>(registry);
    register_message::<Identify>(registry);
    register_message::<ActorIdentity>(registry);
    register_message::<PoisonPill>(registry);
    register_message::<Resume>(registry);
    register_message::<Suspend>(registry);
    register_message::<Terminate>(registry);
    register_message::<Unwatch>(registry);
    register_message::<Watch>(registry);
    register_message::<ActorSelectionMessage>(registry);
    register_message::<ArteryHeartbeat>(registry);
    register_message::<ArteryHeartbeatRsp>(registry);
    register_message::<Heartbeat>(registry);
    register_message::<HeartbeatRsp>(registry);
}

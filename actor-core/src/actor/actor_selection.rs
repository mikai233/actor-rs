use std::any::Any;

use anyhow::anyhow;
use bincode::{Decode, Encode};
use bincode::error::{DecodeError, EncodeError};
use enum_dispatch::enum_dispatch;

use crate::{CodecMessage, DynMessage, FakeActor, Message};
use crate::actor::actor_ref::ActorRef;
use crate::actor::context::ActorContext;
use crate::actor::decoder::MessageDecoder;
use crate::ext::{decode_bytes, encode_bytes};
use crate::message::identify::Identify;
use crate::message::message_registration::{IDPacket, MessageRegistration};

pub struct ActorSelection {
    pub(crate) anchor: ActorRef,
    pub(crate) path: Vec<SelectionPathElement>,
}

impl ActorSelection {
    pub(crate) fn tell(&self, _message: DynMessage, _sender: Option<ActorRef>) {}

    fn deliver_selection(anchor: ActorRef, sender: Option<ActorRef>, sel: ActorSelectionMessage) {
        if sel.elements.is_empty() {
            anchor.tell(sel.message, sender);
        } else {
            fn rec(actor_ref: ActorRef, sel: ActorSelectionMessage, mut iter: impl Iterator<Item=SelectionPathElement>) {
                match actor_ref.local() {
                    None => {
                        sel.copy_with_elements(iter.collect());
                    }
                    Some(local) => {}
                }
            }
        }
    }
}

#[enum_dispatch(SelectionPathElement)]
pub(crate) trait TSelectionPathElement {}

#[enum_dispatch]
#[derive(Debug, Clone, Encode, Decode)]
pub(crate) enum SelectionPathElement {
    SelectChildName,
    SelectChildPattern,
    SelectParent,
}

#[derive(Debug, Clone, Encode, Decode)]
pub(crate) struct SelectChildName {
    name: String,
}

impl TSelectionPathElement for SelectChildName {}

#[derive(Debug, Clone, Encode, Decode)]
pub(crate) struct SelectChildPattern {
    pattern_str: String,
}

impl TSelectionPathElement for SelectChildPattern {}

#[derive(Debug, Clone, Encode, Decode)]
pub(crate) struct SelectParent;

impl TSelectionPathElement for SelectParent {}

#[derive(Debug)]
pub(crate) struct ActorSelectionMessage {
    message: DynMessage,
    elements: Vec<SelectionPathElement>,
    wildcard_fan_out: bool,
}

impl ActorSelectionMessage {
    pub(crate) fn new(message: DynMessage, elements: Vec<SelectionPathElement>, wildcard_fan_out: bool) -> anyhow::Result<Self> {
        if message.dyn_clone().is_none() {
            Err(anyhow!("message {} must be cloneable", message.name()))
        } else {
            let myself = Self {
                message,
                elements,
                wildcard_fan_out,
            };
            Ok(myself)
        }
    }
    pub(crate) fn identify_request(&self) -> anyhow::Result<&Identify> {
        self.message.downcast_as_message::<FakeActor, _>()
    }

    pub(crate) fn copy_with_elements(&self, elements: Vec<SelectionPathElement>) -> Self {
        let Self { message, wildcard_fan_out, .. } = self;
        Self {
            message: message.dyn_clone().unwrap(),
            elements,
            wildcard_fan_out: *wildcard_fan_out,
        }
    }
}

#[derive(Debug, Encode, Decode)]
struct SerdeSelectionMessage {
    packet: IDPacket,
    elements: Vec<SelectionPathElement>,
    wildcard_fan_out: bool,
}

impl CodecMessage for ActorSelectionMessage {
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn decoder() -> Option<Box<dyn MessageDecoder>> where Self: Sized {
        #[derive(Clone)]
        struct D;
        impl MessageDecoder for D {
            fn decode(&self, bytes: &[u8], reg: &MessageRegistration) -> Result<DynMessage, DecodeError> {
                let SerdeSelectionMessage { packet, elements, wildcard_fan_out } = decode_bytes::<SerdeSelectionMessage>(bytes)?;
                let message = reg.decode(packet)?;
                let message = ActorSelectionMessage {
                    message,
                    elements,
                    wildcard_fan_out,
                };
                Ok(DynMessage::user(message))
            }
        }

        Some(Box::new(D))
    }

    fn encode(&self, reg: &MessageRegistration) -> Result<Vec<u8>, EncodeError> {
        let ActorSelectionMessage { message, elements, wildcard_fan_out } = self;
        let packet = reg.encode_boxed(message)?;
        let message = SerdeSelectionMessage {
            packet,
            elements: elements.clone(),
            wildcard_fan_out: *wildcard_fan_out,
        };
        encode_bytes(&message)
    }

    fn dyn_clone(&self) -> Option<DynMessage> {
        self.message.dyn_clone().map(|m| {
            let message = ActorSelectionMessage {
                message: m,
                elements: self.elements.clone(),
                wildcard_fan_out: self.wildcard_fan_out,
            };
            DynMessage::user(message)
        })
    }
}

impl Message for ActorSelectionMessage {
    type A = FakeActor;

    fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        todo!()
    }
}
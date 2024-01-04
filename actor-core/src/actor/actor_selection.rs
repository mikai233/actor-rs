use std::any::Any;
use std::ops::Deref;

use anyhow::anyhow;
use async_trait::async_trait;
use bincode::{BorrowDecode, Decode, Encode};
use bincode::de::{BorrowDecoder, Decoder};
use bincode::enc::Encoder;
use bincode::error::{DecodeError, EncodeError};
use enum_dispatch::enum_dispatch;
use regex::Regex;

use crate::{Actor, CodecMessage, DynMessage, FakeActor, SystemMessage};
use crate::actor::actor_path::TActorPath;
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
        } else {}
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

#[derive(Debug, Clone)]
pub(crate) struct SelectChildPattern {
    pattern: Regex,
}

impl SelectChildPattern {
    fn new(pattern_str: impl Into<String>) -> anyhow::Result<Self> {
        let pattern_str = pattern_str.into();
        let regex = Regex::new(&pattern_str)?;
        Ok(Self {
            pattern: regex,
        })
    }
}

impl Encode for SelectChildPattern {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        Encode::encode(self.pattern.as_str(), encoder)
    }
}

impl Decode for SelectChildPattern {
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
        let pattern_str: String = Decode::decode(decoder)?;
        let pattern = Regex::new(&pattern_str).map_err(|e| DecodeError::OtherString(e.to_string()))?;
        Ok(Self {
            pattern,
        })
    }
}

impl<'de> BorrowDecode<'de> for SelectChildPattern {
    fn borrow_decode<D: BorrowDecoder<'de>>(decoder: &mut D) -> Result<Self, DecodeError> {
        let pattern_str: String = Decode::decode(decoder)?;
        let pattern = Regex::new(&pattern_str).map_err(|e| DecodeError::OtherString(e.to_string()))?;
        Ok(Self {
            pattern,
        })
    }
}

impl Deref for SelectChildPattern {
    type Target = Regex;

    fn deref(&self) -> &Self::Target {
        &self.pattern
    }
}

impl TSelectionPathElement for SelectChildPattern {}

#[derive(Debug, Clone, Encode, Decode)]
pub(crate) struct SelectParent;

impl TSelectionPathElement for SelectParent {}

#[derive(Debug)]
pub(crate) struct ActorSelectionMessage {
    pub(crate) message: DynMessage,
    pub(crate) elements: Vec<SelectionPathElement>,
    pub(crate) wildcard_fan_out: bool,
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
                Ok(DynMessage::system(message))
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
            DynMessage::system(message)
        })
    }
}

#[async_trait]
impl SystemMessage for ActorSelectionMessage {
    async fn handle(self: Box<Self>, _context: &mut ActorContext, _actor: &mut dyn Actor) -> anyhow::Result<()> {
        Ok(())
    }
}
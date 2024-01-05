use std::any::Any;
use std::fmt::{Display, Formatter};
use std::ops::Deref;

use anyhow::anyhow;
use bincode::{BorrowDecode, Decode, Encode};
use bincode::de::{BorrowDecoder, Decoder};
use bincode::enc::Encoder;
use bincode::error::{DecodeError, EncodeError};
use enum_dispatch::enum_dispatch;
use regex::Regex;

use crate::{CodecMessage, DynMessage, OrphanMessage};
use crate::actor::actor_ref::{ActorRef, TActorRef};
use crate::actor::decoder::MessageDecoder;
use crate::actor::empty_local_ref::EmptyLocalActorRef;
use crate::actor::actor_path::TActorPath;
use crate::actor::cell::Cell;
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
            let elements = sel.elements.iter().map(|e| e.to_string()).collect::<Vec<_>>();
            let empty_ref = EmptyLocalActorRef::new(
                anchor.system().clone(),
                anchor.path().descendant(elements.iter().map(|e| e.as_str())),
            );
            let mut iter = sel.elements.iter().peekable();
            let mut actor = &anchor;
            loop {
                match actor.local() {
                    Some(local) => {
                        match iter.next() {
                            Some(element) => {
                                match element {
                                    SelectionPathElement::SelectChildName(c) => {}
                                    SelectionPathElement::SelectChildPattern(p) => {
                                        let matching_children = local.children().iter().filter(|c| {
                                            p.is_match(c.value().path().name())
                                        }).collect::<Vec<_>>();
                                        if iter.peek().is_none() {
                                            if matching_children.is_empty() && !sel.wildcard_fan_out {
                                                empty_ref.tell(DynMessage::orphan(sel.clone()), sender.clone())
                                            } else {
                                                for child in matching_children {
                                                    child.value().tell(sel.message.dyn_clone().unwrap(), sender.clone());
                                                }
                                            }
                                        } else {
                                            if matching_children.is_empty() && !sel.wildcard_fan_out {
                                                empty_ref.tell(DynMessage::orphan(sel.clone()), sender.clone())
                                            } else {
                                                let wildcard_fan_out = sel.wildcard_fan_out || matching_children.len() > 1;
                                                let m = sel.copy_with_elements(iter.map(|e|e.clone()).collect(), wildcard_fan_out);
                                                for child in matching_children {
                                                    Self::deliver_selection(child.value().clone(), sender.clone(), m.clone());
                                                }
                                            }
                                        }
                                        break;
                                    }
                                    SelectionPathElement::SelectParent(p) => {
                                        match local.parent() {
                                            Some(parent) => {
                                                if iter.peek().is_none() {
                                                    parent.tell(sel.message.dyn_clone().unwrap(), sender.clone());
                                                } else {
                                                    actor = parent;
                                                }
                                            }
                                            None => {}
                                        }
                                    }
                                }
                            }
                            None => {}
                        }
                    }
                    None => {}
                }
            }
        }
    }
}

#[enum_dispatch(SelectionPathElement)]
pub(crate) trait TSelectionPathElement: Display {}

#[enum_dispatch]
#[derive(Debug, Clone, Encode, Decode)]
pub(crate) enum SelectionPathElement {
    SelectChildName,
    SelectChildPattern,
    SelectParent,
}

impl Display for SelectionPathElement {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            SelectionPathElement::SelectChildName(c) => {
                Display::fmt(c, f)
            }
            SelectionPathElement::SelectChildPattern(p) => {
                Display::fmt(p, f)
            }
            SelectionPathElement::SelectParent(p) => {
                Display::fmt(p, f)
            }
        }
    }
}

#[derive(Debug, Clone, Encode, Decode)]
pub(crate) struct SelectChildName {
    name: String,
}

impl Display for SelectChildName {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
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

impl Display for SelectChildPattern {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl TSelectionPathElement for SelectChildPattern {}

#[derive(Debug, Clone, Encode, Decode)]
pub(crate) struct SelectParent;

impl Display for SelectParent {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "..")
    }
}

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
    pub(crate) fn identify_request(&self) -> Option<&Identify> {
        self.message.downcast_system_ref()
    }

    pub(crate) fn copy_with_elements(&self, elements: Vec<SelectionPathElement>, wildcard_fan_out: bool) -> Self {
        let Self { message, .. } = self;
        Self {
            message: message.dyn_clone().unwrap(),
            elements,
            wildcard_fan_out,
        }
    }
}

impl Clone for ActorSelectionMessage {
    fn clone(&self) -> Self {
        let Self { message, elements, wildcard_fan_out } = self;
        Self {
            message: message.dyn_clone().unwrap(),
            elements: elements.clone(),
            wildcard_fan_out: *wildcard_fan_out,
        }
    }
}

#[derive(Debug, Encode, Decode)]
struct CodecSelectionMessage {
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
                let CodecSelectionMessage { packet, elements, wildcard_fan_out } = decode_bytes::<CodecSelectionMessage>(bytes)?;
                let message = reg.decode(packet)?;
                let message = ActorSelectionMessage {
                    message,
                    elements,
                    wildcard_fan_out,
                };
                Ok(DynMessage::orphan(message))
            }
        }

        Some(Box::new(D))
    }

    fn encode(&self, reg: &MessageRegistration) -> Result<Vec<u8>, EncodeError> {
        let ActorSelectionMessage { message, elements, wildcard_fan_out } = self;
        let packet = reg.encode_boxed(message)?;
        let message = CodecSelectionMessage {
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
            DynMessage::orphan(message)
        })
    }
}

impl OrphanMessage for ActorSelectionMessage {}
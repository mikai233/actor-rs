use std::any::Any;
use std::fmt::{Display, Formatter};
use std::ops::Deref;
use std::time::Duration;

use anyhow::anyhow;
use bincode::{BorrowDecode, Decode, Encode};
use bincode::de::{BorrowDecoder, Decoder};
use bincode::enc::Encoder;
use bincode::error::{DecodeError, EncodeError};
use enum_dispatch::enum_dispatch;
use regex::Regex;
use tracing::error;

use crate::{CodecMessage, DynMessage, OrphanMessage};
use crate::actor::actor_path::{ActorPath, TActorPath};
use crate::actor::actor_ref::{ActorRef, TActorRef};
use crate::actor::cell::Cell;
use crate::actor::context::{ActorContext, Context};
use crate::actor::decoder::MessageDecoder;
use crate::actor::empty_local_ref::EmptyLocalActorRef;
use crate::ext::{decode_bytes, encode_bytes};
use crate::message::identify::{ActorIdentity, Identify};
use crate::message::message_registration::{IDPacket, MessageRegistration};
use crate::pattern::patterns::Patterns;

#[derive(Debug)]
pub struct ActorSelection {
    pub(crate) anchor: ActorRef,
    pub(crate) path: Vec<SelectionPathElement>,
}

impl ActorSelection {
    pub(crate) fn new<'a>(anchor: ActorRef, elements: impl IntoIterator<Item=&'a str>) -> anyhow::Result<Self> {
        let mut path: Vec<SelectionPathElement> = vec![];
        for e in elements.into_iter() {
            if !e.is_empty() {
                match e {
                    x if x.find('?').is_some() || x.find('*').is_some() => {
                        let pattern = SelectChildPattern::new(x)?;
                        path.push(pattern.into())
                    }
                    x if x == ".." => {
                        path.push(SelectParent.into());
                    }
                    x => {
                        path.push(SelectChildName::new(x.to_string()).into())
                    }
                }
            }
        }
        let sel = Self {
            anchor,
            path,
        };
        Ok(sel)
    }

    pub fn tell(&self, message: DynMessage, sender: Option<ActorRef>) {
        match ActorSelectionMessage::new(message, self.path.clone(), false) {
            Ok(sel) => {
                Self::deliver_selection(self.anchor.clone(), sender, sel);
            }
            Err(error) => {
                error!("{}", error);
            }
        };
    }

    pub fn forward(&self, message: DynMessage, context: &ActorContext) {
        self.tell(message, context.sender().cloned());
    }

    pub async fn resolve_one(&self, timeout: Duration) -> anyhow::Result<ActorRef> {
        let actor_identity: ActorIdentity = Patterns::ask_selection_sys(self, Identify, timeout).await?;
        match actor_identity.actor_ref {
            None => {
                Err(anyhow!("actor not found of selection {}", self))
            }
            Some(actor_ref) => {
                Ok(actor_ref)
            }
        }
    }

    pub(crate) fn path_str(&self) -> String {
        self.path.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("/")
    }

    pub(crate) fn deliver_selection(anchor: ActorRef, sender: Option<ActorRef>, sel: ActorSelectionMessage) {
        if sel.elements.is_empty() {
            anchor.tell(sel.message, sender);
        } else {
            let elements = sel.elements.iter().map(|e| e.to_string()).collect::<Vec<_>>();
            let empty_ref = EmptyLocalActorRef::new(
                anchor.system().clone(),
                anchor.path().descendant(elements.iter().map(|e| e.as_str())),
            );
            let mut iter = sel.elements.iter().peekable();
            let mut actor = anchor;
            loop {
                match actor.local() {
                    Some(local) => {
                        match iter.next() {
                            Some(element) => {
                                match element {
                                    SelectionPathElement::SelectChildName(c) => {
                                        match local.get_single_child(&c.name) {
                                            Some(child) => {
                                                if iter.peek().is_none() {
                                                    child.tell(sel.message, sender);
                                                    break;
                                                } else {
                                                    actor = child;
                                                }
                                            }
                                            None => {
                                                if !sel.wildcard_fan_out {
                                                    empty_ref.tell(DynMessage::orphan(sel), sender);
                                                }
                                                break;
                                            }
                                        }
                                    }
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
                                                let m = sel.copy_with_elements(iter.map(|e| e.clone()).collect(), wildcard_fan_out);
                                                for child in matching_children {
                                                    Self::deliver_selection(child.value().clone(), sender.clone(), m.clone());
                                                }
                                            }
                                        }
                                        break;
                                    }
                                    SelectionPathElement::SelectParent(_) => {
                                        match local.parent() {
                                            Some(parent) => {
                                                if iter.peek().is_none() {
                                                    parent.tell(sel.message.dyn_clone().unwrap(), sender.clone());
                                                    break;
                                                } else {
                                                    actor = parent.clone();
                                                }
                                            }
                                            None => {
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                            None => {
                                break;
                            }
                        }
                    }
                    None => {
                        let sel = sel.copy_with_elements(iter.map(|e| e.clone()).collect(), sel.wildcard_fan_out);
                        actor.tell(DynMessage::orphan(sel), sender);
                        break;
                    }
                }
            }
        }
    }
}

impl Display for ActorSelection {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}[{}]", self.anchor, self.path_str())
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

impl SelectChildName {
    pub(crate) fn new(name: String) -> Self {
        Self {
            name,
        }
    }
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

#[derive(Debug, Clone)]
pub enum ActorSelectionPath {
    RelativePath(String),
    FullPath(ActorPath),
}

#[derive(Debug)]
pub(crate) struct ActorSelectionMessage {
    pub(crate) message: DynMessage,
    pub(crate) elements: Vec<SelectionPathElement>,
    pub(crate) wildcard_fan_out: bool,
}

impl ActorSelectionMessage {
    pub(crate) fn new(message: DynMessage, elements: Vec<SelectionPathElement>, wildcard_fan_out: bool) -> anyhow::Result<Self> {
        if message.is_cloneable() {
            let myself = Self {
                message,
                elements,
                wildcard_fan_out,
            };
            Ok(myself)
        } else {
            Err(anyhow!("message {} must be cloneable", message.name()))
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

    fn dyn_clone(&self) -> anyhow::Result<DynMessage> {
        self.message.dyn_clone().map(|m| {
            let message = ActorSelectionMessage {
                message: m,
                elements: self.elements.clone(),
                wildcard_fan_out: self.wildcard_fan_out,
            };
            DynMessage::orphan(message)
        })
    }

    fn is_cloneable(&self) -> bool {
        self.message.message.is_cloneable()
    }
}

impl OrphanMessage for ActorSelectionMessage {}
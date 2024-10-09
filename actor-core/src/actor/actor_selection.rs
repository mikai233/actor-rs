use std::any::Any;
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::time::Duration;

use crate::actor::context::{Context, ActorContext};
use crate::actor_path::ActorPath;
use crate::actor_path::TActorPath;
use crate::actor_ref::empty_local_ref::EmptyLocalActorRef;
use crate::actor_ref::{ActorRef, TActorRef};
use crate::cell::Cell;
use crate::message::identify::{ActorIdentity, Identify};
use crate::message::{downcast_ref, DynMessage, Message, Signature};
use crate::pattern::patterns::Patterns;
use anyhow::anyhow;
use enum_dispatch::enum_dispatch;
use itertools::Itertools;
use regex::Regex;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use tracing::{error, warn};

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct ActorSelection {
    pub(crate) anchor: ActorRef,
    pub(crate) path: Vec<SelectionPathElement>,
}

impl ActorSelection {
    pub(crate) fn new<'a>(
        anchor: ActorRef,
        elements: impl IntoIterator<Item = &'a str>,
    ) -> anyhow::Result<Self> {
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
                    x => path.push(SelectChildName::new(x.to_string()).into()),
                }
            }
        }
        let sel = Self { anchor, path };
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

    pub async fn resolve_one(&self, timeout: Duration) -> anyhow::Result<ActorRef> {
        let actor_identity: ActorIdentity =
            Patterns::ask_selection_sys(self, Identify, timeout).await?;
        match actor_identity.actor_ref {
            None => Err(anyhow!("actor not found of selection {}", self)),
            Some(actor_ref) => Ok(actor_ref),
        }
    }

    pub(crate) fn path_str(&self) -> String {
        self.path
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("/")
    }

    pub(crate) fn deliver_selection(
        anchor: ActorRef,
        sender: Option<ActorRef>,
        sel: ActorSelectionMessage,
    ) {
        if sel.elements.is_empty() {
            anchor.tell(sel.message, sender);
        } else {
            let elements = sel
                .elements
                .iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>();
            let empty_ref = EmptyLocalActorRef::new(
                anchor.system().clone(),
                anchor
                    .path()
                    .descendant(elements.iter().map(|e| e.as_str())),
            );
            let mut iter = sel.elements.iter().peekable();
            let mut actor = anchor;
            loop {
                match actor.local() {
                    Some(local) => match iter.next() {
                        Some(element) => match element {
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
                                            empty_ref.tell(sel, sender);
                                        }
                                        break;
                                    }
                                }
                            }
                            SelectionPathElement::SelectChildPattern(p) => {
                                let matching_children = local
                                    .children()
                                    .iter()
                                    .filter(|c| p.is_match(c.value().path().name()))
                                    .collect::<Vec<_>>();
                                if iter.peek().is_none() {
                                    if matching_children.is_empty() && !sel.wildcard_fan_out {
                                        empty_ref.tell(sel, sender.clone())
                                    } else {
                                        for child in matching_children {
                                            let message = sel.message.clone_box().expect("message is not cloneable");
                                            child.value().tell(message, sender.clone());
                                        }
                                    }
                                } else {
                                    if matching_children.is_empty() && !sel.wildcard_fan_out {
                                        empty_ref.tell(sel, sender.clone());
                                    } else {
                                        let wildcard_fan_out =
                                            sel.wildcard_fan_out || matching_children.len() > 1;
                                        let m = sel.copy_with_elements(
                                            iter.cloned().collect(),
                                            wildcard_fan_out,
                                        );
                                        for child in matching_children {
                                            let m = ActorSelectionMessage {
                                                message: m.message.clone_box().unwrap(),
                                                elements: m.elements.clone(),
                                                wildcard_fan_out: m.wildcard_fan_out,
                                            };
                                            Self::deliver_selection(
                                                child.value().clone(),
                                                sender.clone(),
                                                m,
                                            );
                                        }
                                    }
                                }
                                break;
                            }
                            SelectionPathElement::SelectParent(_) => match local.parent() {
                                Some(parent) => {
                                    if iter.peek().is_none() {
                                        parent
                                            .tell(sel.message.dyn_clone().unwrap(), sender.clone());
                                        break;
                                    } else {
                                        actor = parent.clone();
                                    }
                                }
                                None => {
                                    break;
                                }
                            },
                        },
                        None => {
                            break;
                        }
                    },
                    None => {
                        let sel = sel.copy_with_elements(
                            iter.cloned().collect(),
                            sel.wildcard_fan_out,
                        );
                        actor.tell(sel, sender);
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
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub(crate) enum SelectionPathElement {
    SelectChildName,
    SelectChildPattern,
    SelectParent,
}

impl Display for SelectionPathElement {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            SelectionPathElement::SelectChildName(c) => Display::fmt(c, f),
            SelectionPathElement::SelectChildPattern(p) => Display::fmt(p, f),
            SelectionPathElement::SelectParent(p) => Display::fmt(p, f),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub(crate) struct SelectChildName {
    name: String,
}

impl SelectChildName {
    pub(crate) fn new(name: String) -> Self {
        Self { name }
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
        Ok(Self { pattern: regex })
    }
}

impl Serialize for SelectChildPattern {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.pattern.as_str())
    }
}

impl<'de> Deserialize<'de> for SelectChildPattern {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let pattern_str = String::deserialize(deserializer)?;
        Ok(Self::new(pattern_str)?)
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

impl PartialEq for SelectChildPattern {
    fn eq(&self, other: &Self) -> bool {
        self.as_str().eq(other.as_str())
    }
}

impl Eq for SelectChildPattern {}

impl Hash for SelectChildPattern {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_str().hash(state)
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub(crate) struct SelectParent;

impl Display for SelectParent {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "..")
    }
}

impl TSelectionPathElement for SelectParent {}

//TODO 更加友好的API
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
    pub(crate) fn new(
        message: DynMessage,
        elements: Vec<SelectionPathElement>,
        wildcard_fan_out: bool,
    ) -> anyhow::Result<Self> {
        if message.cloneable() {
            let myself = Self {
                message,
                elements,
                wildcard_fan_out,
            };
            Ok(myself)
        } else {
            Err(anyhow!("message {} must be cloneable", message.signature()))
        }
    }
    pub(crate) fn identify_request(&self) -> Option<&Identify> {
        downcast_ref(&self.message)
    }

    pub(crate) fn copy_with_elements(
        &self,
        elements: Vec<SelectionPathElement>,
        wildcard_fan_out: bool,
    ) -> Self {
        let Self { message, .. } = self;
        let message = message.clone_box().expect("message is not cloneable");
        Self {
            message, 
            elements,
            wildcard_fan_out,
        }
    }
}

impl Message for ActorSelectionMessage {
    fn signature_sized() -> Signature
    where
        Self: Sized,
    {
        Signature::new::<Self>()
    }

    fn signature(&self) -> Signature {
        Signature::new::<Self>()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    fn is_cloneable(&self) -> bool {
        self.message.is_cloneable()
    }

    fn clone_box(&self) -> Option<Box<dyn Message>> {
        if self.is_cloneable() {
            let m = Self {
                message: self.message.clone_box().unwrap(),
                elements: self.elements.clone(),
                wildcard_fan_out: self.wildcard_fan_out,
            };
            Some(Box::new(m))
        } else {
            None
        }
    }
}

impl Display for ActorSelectionMessage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ActorSelectionMessage {{ message: {}, elements: {}, wildcard_fan_out: {} }}",
            self.message,
            self.elements.iter().join(", "),
            self.wildcard_fan_out,
        )
    }
}

use anyhow::anyhow;
use enum_dispatch::enum_dispatch;

use crate::{DynMessage, FakeActor};
use crate::actor::actor_ref::ActorRef;
use crate::message::identify::Identify;

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
            fn rec(actor_ref: ActorRef, mut iter: impl Iterator<Item=SelectionPathElement>) {
                match actor_ref.local() {
                    None => {
                        actor_ref.tell()
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
#[derive(Debug, Clone)]
pub(crate) enum SelectionPathElement {
    SelectChildName,
    SelectChildPattern,
    SelectParent,
}

#[derive(Debug, Clone)]
pub(crate) struct SelectChildName {
    name: String,
}

impl TSelectionPathElement for SelectChildName {}

#[derive(Debug, Clone)]
pub(crate) struct SelectChildPattern {
    pattern_str: String,
}

impl TSelectionPathElement for SelectChildPattern {}

#[derive(Debug, Clone)]
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
        if message.clone().is_none() {
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
            message: message.clone().unwrap(),
            elements,
            wildcard_fan_out: *wildcard_fan_out,
        }
    }
}
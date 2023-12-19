use enum_dispatch::enum_dispatch;

use crate::actor::actor_ref::ActorRef;
use crate::DynMessage;
use crate::message::identify::Identify;

pub struct ActorSelection {
    pub(crate) anchor: ActorRef,
    pub(crate) path: Vec<SelectionPathElement>,
}

impl ActorSelection {
    pub(crate) fn tell(&self, message: DynMessage, sender: Option<ActorRef>) {}

    fn deliver_selection(anchor: ActorRef, sender: Option<ActorRef>, sel: ActorSelectionMessage) {
        if sel.elements.is_empty() {
            anchor.tell(sel.message, sender);
        } else {
            todo!()
        }
    }
}

#[enum_dispatch(SelectionPathElement)]
pub(crate) trait TSelectionPathElement {}

#[enum_dispatch]
pub(crate) enum SelectionPathElement {
    SelectChildName,
    SelectChildPattern,
    SelectParent,
}

pub(crate) struct SelectChildName {
    name: String,
}

impl TSelectionPathElement for SelectChildName {}

pub(crate) struct SelectChildPattern {
    pattern_str: String,
}

impl TSelectionPathElement for SelectChildPattern {}

pub(crate) struct SelectParent;

impl TSelectionPathElement for SelectParent {}

pub(crate) struct ActorSelectionMessage {
    message: DynMessage,
    elements: Vec<SelectionPathElement>,
    wildcard_fan_out: bool,
}

impl ActorSelectionMessage {
    pub(crate) fn identify_request(&self) -> Option<Identify> {
        todo!()
    }
}
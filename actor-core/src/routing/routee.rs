use std::fmt::Debug;
use std::ops::Deref;

use dyn_clone::DynClone;
use tracing::error;

use crate::actor::actor_ref::ActorRef;
use crate::actor::actor_selection::ActorSelection;
use crate::DynMessage;

pub trait Routee: Send + DynClone + Debug {
    fn send(&self, message: DynMessage, sender: Option<ActorRef>);
}

dyn_clone::clone_trait_object!(Routee);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActorRefRoutee(pub ActorRef);

impl Deref for ActorRefRoutee {
    type Target = ActorRef;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Routee for ActorRefRoutee {
    fn send(&self, message: DynMessage, sender: Option<ActorRef>) {
        self.tell(message, sender);
    }
}

#[derive(Debug, Clone)]
pub struct ActorSelectionRoutee(pub ActorSelection);

impl Deref for ActorSelectionRoutee {
    type Target = ActorSelection;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Routee for ActorSelectionRoutee {
    fn send(&self, message: DynMessage, sender: Option<ActorRef>) {
        self.tell(message, sender);
    }
}

#[derive(Debug, Clone)]
pub struct NoRoutee;

impl Routee for NoRoutee {
    fn send(&self, _message: DynMessage, _sender: Option<ActorRef>) {}
}

//TODO reference
#[derive(Debug, Clone)]
pub struct SeveralRoutees {
    pub routees: Vec<Box<dyn Routee>>,
}

impl Deref for SeveralRoutees {
    type Target = Vec<Box<dyn Routee>>;

    fn deref(&self) -> &Self::Target {
        &self.routees
    }
}

impl Routee for SeveralRoutees {
    fn send(&self, message: DynMessage, sender: Option<ActorRef>) {
        for routee in &self.routees {
            match message.dyn_clone() {
                Ok(message) => {
                    routee.send(message, sender.clone());
                }
                Err(_) => {
                    error!("route message {} not impl dyn_clone", message.name())
                }
            }
        }
    }
}

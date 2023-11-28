use std::ops::Deref;

use dyn_clone::DynClone;
use tracing::error;

use crate::actor_ref::ActorRef;
use crate::actor_ref::TActorRef;
use crate::DynMessage;

pub struct Router {
    logic: Box<dyn RoutingLogic>,
    routees: Vec<Box<dyn Routee>>,
}

pub trait RoutingLogic: Send + 'static {
    fn select(&self, message: DynMessage, routees: &Vec<Box<dyn Routee>>) -> Box<dyn Routee>;
}

pub trait Routee: Send + DynClone + 'static {
    fn send(&self, message: DynMessage, sender: Option<ActorRef>);
}

dyn_clone::clone_trait_object!(Routee);

#[derive(Debug, Clone)]
pub struct ActorRefRoutee(ActorRef);

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
pub struct ActorSelectionRoutee;//TODO

#[derive(Debug, Clone)]
pub struct NoRoutee;

impl Routee for NoRoutee {
    fn send(&self, _message: DynMessage, _sender: Option<ActorRef>) {}
}

#[derive(Clone)]
pub struct SeveralRoutees {
    pub routees: Vec<Box<dyn Routee>>,
}

impl Routee for SeveralRoutees {
    fn send(&self, message: DynMessage, sender: Option<ActorRef>) {
        for routee in &self.routees {
            match message.clone() {
                None => {
                    error!("route message {} not impl dyn_clone", message.name())
                }
                Some(message) => {
                    routee.send(message, sender.clone());
                }
            }
        }
    }
}
use std::ops::Deref;
use std::sync::Arc;

use tracing::error;

use actor_derive::EmptyCodec;

use crate::actor::actor_ref::ActorRef;
use crate::DynMessage;

pub struct Router {
    logic: Box<dyn RoutingLogic>,
    routees: Vec<Arc<Box<dyn Routee>>>,
}

impl Router {
    pub fn new<L>(logic: L) -> Self where L: RoutingLogic {
        Self {
            logic: Box::new(logic),
            routees: Vec::new(),
        }
    }

    pub fn new_with_routees<L>(logic: L, routees: Vec<Box<dyn Routee>>) -> Self where L: RoutingLogic {
        Self {
            logic: Box::new(logic),
            routees: routees.into_iter().map(Into::into).collect(),
        }
    }

    pub fn route(&self, message: DynMessage, sender: Option<ActorRef>) {
        if message.boxed.as_any().is::<Broadcast>() {
            let broadcast = message.boxed.into_any().downcast::<Broadcast>().unwrap();
            let message = broadcast.message;
            SeveralRoutees { routees: self.routees.clone() }.send(message, sender);
        } else {
            self.send(&self.logic.select(&message, &self.routees), message, sender);
        }
    }

    fn send(&self, _routee: &Box<dyn Routee>, _message: DynMessage, _sender: Option<ActorRef>) {}
}

pub trait RoutingLogic: Send + Sync + 'static {
    fn select(&self, message: &DynMessage, routees: &Vec<Arc<Box<dyn Routee>>>) -> Arc<Box<dyn Routee>>;
}

pub trait Routee: Send + Sync + 'static {
    fn send(&self, message: DynMessage, sender: Option<ActorRef>);
}

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

pub struct SeveralRoutees {
    pub routees: Vec<Arc<Box<dyn Routee>>>,
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

#[derive(Debug, EmptyCodec)]
pub struct Broadcast {
    message: DynMessage,
}
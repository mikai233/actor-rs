use std::any::Any;
use std::ops::Deref;
use std::sync::Arc;

use dyn_clone::DynClone;
use tracing::error;

use actor_derive::{AsAny, EmptyCodec};

use crate::actor::actor_ref::ActorRef;
use crate::actor::actor_ref_factory::ActorRefFactory;
use crate::actor::actor_system::ActorSystem;
use crate::DynMessage;
use crate::ext::as_any::AsAny;

pub enum MaybeRef<'a, T> {
    Ref(&'a T),
    Own(T),
}

impl<'a, T> Deref for MaybeRef<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            MaybeRef::Ref(value) => value,
            MaybeRef::Own(value) => value,
        }
    }
}

#[derive(Clone)]
pub struct Router {
    pub logic: Box<dyn RoutingLogic>,
    pub routees: Vec<Arc<Box<dyn Routee>>>,
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

    pub fn route(&self, system: &ActorSystem, message: DynMessage, sender: Option<ActorRef>) {
        if message.boxed.as_any().is::<Broadcast>() {
            let broadcast = message.boxed.into_any().downcast::<Broadcast>().unwrap();
            let message = broadcast.message;
            SeveralRoutees { routees: self.routees.clone() }.send(message, sender);
        } else {
            self.send(&system, &self.logic.select(&message, &self.routees), message, sender);
        }
    }

    fn send(&self, system: &ActorSystem, routee: &Box<dyn Routee>, message: DynMessage, sender: Option<ActorRef>) {
        if routee.as_any().is::<NoRoutee>() {
            let provider = system.provider();
            let dead_letters = provider.dead_letters();
            dead_letters.tell(message, sender);
        } else {
            routee.send(message, sender);
        }
    }

    pub fn with_routees(&self, routees: Vec<Arc<Box<dyn Routee>>>) -> Self {
        let Self { logic, .. } = self;
        Self {
            logic: logic.clone(),
            routees,
        }
    }

    pub fn add_routee(&self, routee: Arc<Box<dyn Routee>>) -> Self {
        let Self { logic, routees } = self;
        let mut routees = routees.clone();
        routees.push(routee);
        Self {
            logic: logic.clone(),
            routees,
        }
    }

    pub fn remove_routee(&self, routee: Arc<Box<dyn Routee>>) -> Self {
        let Self { logic, routees } = self;
        let mut routees = routees.clone();
        routees.retain(|r| !Arc::ptr_eq(r, &routee));
        Self {
            logic: logic.clone(),
            routees,
        }
    }
}

pub trait RoutingLogic: Send + Sync + DynClone + 'static {
    fn select<'a>(&self, message: &DynMessage, routees: &'a Vec<Arc<Box<dyn Routee>>>) -> MaybeRef<'a, Box<dyn Routee>>;
}

dyn_clone::clone_trait_object!(RoutingLogic);

pub trait Routee: Send + Sync + Any + AsAny {
    fn send(&self, message: DynMessage, sender: Option<ActorRef>);
}

#[derive(Debug, Clone, AsAny)]
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
pub struct ActorSelectionRoutee;//TODO

#[derive(Debug, Clone, AsAny)]
pub struct NoRoutee;

impl Routee for NoRoutee {
    fn send(&self, _message: DynMessage, _sender: Option<ActorRef>) {}
}

#[derive(Clone, AsAny)]
pub struct SeveralRoutees {
    pub routees: Vec<Arc<Box<dyn Routee>>>,
}

impl Routee for SeveralRoutees {
    fn send(&self, message: DynMessage, sender: Option<ActorRef>) {
        for routee in &self.routees {
            match message.dyn_clone() {
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
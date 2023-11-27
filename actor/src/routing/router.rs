use crate::actor_ref::ActorRef;
use crate::DynamicMessage;

pub struct Router {
    logic: Box<dyn RoutingLogic>,
    routees: Vec<Box<dyn Routee>>,
}

pub trait RoutingLogic: Send + 'static {
    fn select(&self, message: DynamicMessage, routees: &Vec<Box<dyn Routee>>) -> &Box<dyn Routee>;
}

pub trait Routee: Send + 'static {
    fn send(&self, message: DynamicMessage, sender: Option<ActorRef>);
}

pub struct SeveralRoutees {
    routees: Vec<Box<dyn Routee>>,
}

impl Routee for SeveralRoutees {
    fn send(&self, message: DynamicMessage, sender: Option<ActorRef>) {
        todo!()
    }
}
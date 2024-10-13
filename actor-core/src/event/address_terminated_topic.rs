use std::any::type_name;
use std::sync::Arc;

use dashmap::DashSet;

use actor_derive::AsAny;

use crate::actor::actor_system::ActorSystem;
use crate::actor::extension::Extension;
use crate::actor_ref::{ActorRef, ActorRefExt};
use crate::message::address_terminated::AddressTerminated;

#[derive(Debug, Clone, Default, AsAny, derive_more::Deref)]
pub struct AddressTerminatedTopic(Arc<AddressTerminatedTopicInner>);

#[derive(Debug, Default)]
pub struct AddressTerminatedTopicInner {
    subscribers: DashSet<ActorRef>,
}

impl AddressTerminatedTopic {
    pub fn new() -> Self {
        AddressTerminatedTopic::default()
    }

    pub fn get(system: &ActorSystem) -> Self {
        system
            .get_extension::<Self>()
            .expect(&format!("{} not found", type_name::<Self>()))
    }

    pub fn subscribe(&self, subscriber: ActorRef) {
        self.subscribers.insert(subscriber);
    }

    pub fn unsubscribe(&self, subscriber: &ActorRef) {
        self.subscribers.remove(subscriber);
    }

    pub fn publish(&self, msg: AddressTerminated) {
        for subscriber in self.subscribers.iter() {
            subscriber.cast_ns(msg.clone());
        }
    }
}

impl Extension for AddressTerminatedTopic {}

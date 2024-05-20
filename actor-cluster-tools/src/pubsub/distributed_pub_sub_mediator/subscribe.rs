use std::marker::PhantomData;

use anyhow::ensure;

use actor_core::Actor;
use actor_core::actor_ref::ActorRef;

#[derive(Debug)]
pub struct Subscribe<A: Actor> {
    topic: String,
    group: Option<String>,
    subscriber: ActorRef,
    _phantom: PhantomData<A>,
}

impl<A> Subscribe<A> where A: Actor {
    pub fn new(topic: String, subscriber: ActorRef) -> anyhow::Result<Self> {
        ensure!(topic.len() > 0, "topic must not be empty");
        let subscribe = Self {
            topic,
            group: None,
            subscriber,
            _phantom: Default::default(),
        };
        Ok(subscribe)
    }

    pub fn new_with_group(topic: String, group: String, subscriber: ActorRef) -> anyhow::Result<Self> {
        ensure!(topic.len() > 0, "topic must not be empty");
        ensure!(group.len() > 0, "group must not be empty");
        let subscribe = Self {
            topic,
            group: Some(group),
            subscriber,
            _phantom: Default::default(),
        };
        Ok(subscribe)
    }

    pub fn topic(&self) -> &str {
        &self.topic
    }

    pub fn group(&self) -> Option<&str> {
        self.group.as_deref()
    }

    pub fn subscriber(&self) -> &ActorRef {
        &self.subscriber
    }
}
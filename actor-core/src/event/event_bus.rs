use std::collections::{HashMap, HashSet};
use std::ops::Not;

use tracing::{debug, trace};

use actor_derive::EmptyCodec;

use crate::{Actor, DynMessage, Message};
use crate::actor::actor_ref::ActorRefExt;
use crate::actor::actor_ref::ActorRef;
use crate::actor::context::{ActorContext, Context};
use crate::event::EventBus;
use crate::message::terminated::WatchTerminated;
use crate::props::Props;
use crate::system::ActorSystem;

#[derive(Debug, Clone)]
pub struct SystemEventBus {
    event_bus: ActorRef,
}

impl SystemEventBus {
    pub(crate) fn new(system: &ActorSystem) -> anyhow::Result<Self> {
        let event_bus = system.system_actor_of(Props::create(|_| EventBusActor::default()), Some("system_event_bus".to_string()))?;
        Ok(Self {
            event_bus,
        })
    }
}

impl EventBus for SystemEventBus {
    type Event = DynMessage;
    type Classifier = &'static str;
    type Subscriber = ActorRef;

    fn subscribe(&self, subscriber: Self::Subscriber, to: Self::Classifier) {
        self.event_bus.cast(Subscribe { subscriber, to }, ActorRef::no_sender());
    }

    fn unsubscribe(&self, subscriber: Self::Subscriber, from: Self::Classifier) {
        self.event_bus.cast(Unsubscribe { subscriber, from }, ActorRef::no_sender());
    }

    fn unsubscribe_all(&self, subscriber: Self::Subscriber) {
        self.event_bus.cast(UnsubscribeAll { subscriber }, ActorRef::no_sender());
    }

    fn publish(&self, event_factory: impl Fn() -> Self::Event + Send + 'static) {
        self.event_bus.cast(Publish { event_factory: Box::new(event_factory) }, ActorRef::no_sender());
    }
}

#[derive(Default)]
struct EventBusActor {
    subscribers: HashMap<&'static str, HashSet<ActorRef>>,
}

impl Actor for EventBusActor {}

#[derive(Debug, EmptyCodec)]
struct WatchSubscriberTerminated {
    watch: ActorRef,
}

impl WatchTerminated for WatchSubscriberTerminated {
    fn watch_actor(&self) -> &ActorRef {
        &self.watch
    }
}

impl Message for WatchSubscriberTerminated {
    type A = EventBusActor;

    fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
        debug!("{} watch subscriber {} terminate, unsubscribe all events", context.myself, self.watch);
        context.myself().cast(UnsubscribeAll { subscriber: self.watch }, ActorRef::no_sender());
        Ok(())
    }
}

#[derive(Debug, EmptyCodec)]
struct Subscribe {
    subscriber: ActorRef,
    to: &'static str,
}

impl Message for Subscribe {
    type A = EventBusActor;

    fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let Subscribe { subscriber, to } = *self;
        debug!("subscriber {} subscribe event {}", subscriber, to);
        if context.is_watching(&subscriber).not() {
            let watch = WatchSubscriberTerminated {
                watch: subscriber.clone(),
            };
            context.watch(watch);
            debug!("watch subscriber {} to {}", subscriber, to);
        }
        let subscribers = actor.subscribers.entry(to).or_insert(HashSet::new());
        subscribers.insert(subscriber);
        Ok(())
    }
}

#[derive(Debug, EmptyCodec)]
struct Unsubscribe {
    subscriber: ActorRef,
    from: &'static str,
}

impl Message for Unsubscribe {
    type A = EventBusActor;

    fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let Unsubscribe { subscriber, from } = *self;
        if let Some(subscribers) = actor.subscribers.get_mut(from) {
            subscribers.remove(&subscriber);
            let all_subscribers = actor.subscribers.values().flat_map(|s| s.iter()).collect::<Vec<&ActorRef>>();
            if all_subscribers.into_iter().all(|s| s != &subscriber) {
                context.unwatch(&subscriber);
                debug!("unwatch subscriber {} form {}", subscriber, from);
            }
            debug!("remove subscriber {} from {}", subscriber, from);
        }
        Ok(())
    }
}

#[derive(Debug, EmptyCodec)]
struct UnsubscribeAll {
    subscriber: ActorRef,
}

impl Message for UnsubscribeAll {
    type A = EventBusActor;

    fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        for subscribers in actor.subscribers.values_mut() {
            subscribers.retain(|s| s != &self.subscriber);
        }
        context.unwatch(&self.subscriber);
        debug!("remove subscriber {} from all events", self.subscriber);
        Ok(())
    }
}

#[derive(EmptyCodec)]
struct Publish {
    event_factory: Box<dyn Fn() -> DynMessage + Send>,
}

impl Message for Publish {
    type A = EventBusActor;

    fn handle(self: Box<Self>, _context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let event = (self.event_factory)();
        let name = event.name();
        if let Some(subscribers) = actor.subscribers.get(name) {
            for subscriber in subscribers {
                let event = (self.event_factory)();
                subscriber.tell(event, ActorRef::no_sender());
                trace!("publish event {} to subscriber {}", name, subscriber);
            }
        }
        Ok(())
    }
}

// #[cfg(test)]
// mod actor_event_bus_test {
//     use std::any::type_name;
//     use std::time::Duration;
//
//     use tracing::info;
//
//     use actor_derive::EmptyCodec;
//
//     use crate::{DynMessage, EmptyTestActor, Message};
//     use crate::actor::context::{ActorContext, Context};
//     use crate::event::event_bus::{EventBusActor, SystemEventBus};
//     use crate::event::EventBus;
//     use crate::props::Props;
//     use crate::system::ActorSystem;
//     use crate::system::config::ActorSystemConfig;
//
//     #[derive(Debug, EmptyCodec)]
//     struct EventMessage1;
//
//     impl Message for EventMessage1 {
//         type A = EmptyTestActor;
//
//         fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
//             info!("{} handle event message {:?}", context.myself(), self);
//             Ok(())
//         }
//     }
//
//     #[derive(Debug, EmptyCodec)]
//     struct EventMessage2;
//
//     impl Message for EventMessage2 {
//         type A = EmptyTestActor;
//
//         fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
//             info!("{} handle event message {:?}", context.myself(), self);
//             Ok(())
//         }
//     }
//
//     #[tokio::test]
//     async fn test_event_bus() -> anyhow::Result<()> {
//         let system = ActorSystem::create(ActorSystemConfig::default()).await?;
//         let props = Props::create(|_| EmptyTestActor);
//         let actor1 = system.spawn_actor(props.clone(), "actor1")?;
//         let actor2 = system.spawn_actor(props.clone(), "actor2")?;
//         let actor3 = system.spawn_actor(props.clone(), "actor3")?;
//         let event_bus_actor = system.spawn_anonymous_actor(Props::create(|_| EventBusActor::default()))?;
//         let event_bus = SystemEventBus {
//             event_bus: event_bus_actor,
//         };
//         event_bus.subscribe(actor1.clone(), type_name::<EventMessage1>());
//         event_bus.subscribe(actor2.clone(), type_name::<EventMessage1>());
//         event_bus.subscribe(actor3.clone(), type_name::<EventMessage1>());
//         event_bus.subscribe(actor3.clone(), type_name::<EventMessage2>());
//         event_bus.publish(|| DynMessage::user(EventMessage1));
//         event_bus.publish(|| DynMessage::user(EventMessage2));
//         event_bus.unsubscribe(actor1, type_name::<EventMessage1>());
//         event_bus.publish(|| DynMessage::user(EventMessage1));
//         event_bus.unsubscribe_all(actor3);
//         event_bus.publish(|| DynMessage::user(EventMessage2));
//         tokio::time::sleep(Duration::from_secs(1)).await;
//         actor2.stop();
//         tokio::time::sleep(Duration::from_secs(1)).await;
//         event_bus.publish(|| DynMessage::user(EventMessage1));
//         tokio::time::sleep(Duration::from_secs(2)).await;
//         Ok(())
//     }
// }
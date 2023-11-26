use std::collections::{HashMap, HashSet};
use std::ops::Not;

use async_trait::async_trait;
use tracing::{debug, trace};

use actor_derive::EmptyCodec;

use crate::{Actor, DynamicMessage, Message};
use crate::actor_ref::{ActorRef, ActorRefExt};
use crate::actor_ref::TActorRef;
use crate::context::{ActorContext, Context};
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
        let event_bus = system.system_actor_of(EventBusActor, (), Some("system_event_bus".to_string()), Props::default())?;
        Ok(Self {
            event_bus,
        })
    }
}

impl EventBus for SystemEventBus {
    type Event = DynamicMessage;
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

struct EventBusActor;

#[async_trait]
impl Actor for EventBusActor {
    type S = HashMap<&'static str, HashSet<ActorRef>>;
    type A = ();

    async fn pre_start(&self, _context: &mut ActorContext, _arg: Self::A) -> anyhow::Result<Self::S> {
        Ok(HashMap::new())
    }
}

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
    type T = EventBusActor;

    fn handle(self: Box<Self>, context: &mut ActorContext, _state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
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
    type T = EventBusActor;

    fn handle(self: Box<Self>, context: &mut ActorContext, state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
        let Subscribe { subscriber, to } = *self;
        debug!("subscriber {} subscribe event {}", subscriber, to);
        if context.is_watching(&subscriber).not() {
            let watch = WatchSubscriberTerminated {
                watch: subscriber.clone(),
            };
            context.watch(watch);
            debug!("watch subscriber {} to {}", subscriber, to);
        }
        let subscribers = state.entry(to).or_insert(HashSet::new());
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
    type T = EventBusActor;

    fn handle(self: Box<Self>, context: &mut ActorContext, state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
        let Unsubscribe { subscriber, from } = *self;
        if let Some(subscribers) = state.get_mut(from) {
            subscribers.remove(&subscriber);
            let all_subscribers = state.values().flat_map(|s| s.iter()).collect::<Vec<&ActorRef>>();
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
    type T = EventBusActor;

    fn handle(self: Box<Self>, context: &mut ActorContext, state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
        for subscribers in state.values_mut() {
            subscribers.retain(|s| s != &self.subscriber);
        }
        context.unwatch(&self.subscriber);
        debug!("remove subscriber {} from all events", self.subscriber);
        Ok(())
    }
}

#[derive(EmptyCodec)]
struct Publish {
    event_factory: Box<dyn Fn() -> DynamicMessage + Send>,
}

impl Message for Publish {
    type T = EventBusActor;

    fn handle(self: Box<Self>, _context: &mut ActorContext, state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
        let event = (self.event_factory)();
        let name = event.name();
        if let Some(subscribers) = state.get(name) {
            for subscriber in subscribers {
                let event = (self.event_factory)();
                subscriber.tell(event, ActorRef::no_sender());
                trace!("publish event {} to subscriber {}", name, subscriber);
            }
        }
        Ok(())
    }
}

mod actor_event_bus_test {
    use std::any::type_name;
    use std::time::Duration;

    use tracing::info;

    use actor_derive::EmptyCodec;

    use crate::{Actor, DynamicMessage, EmptyTestActor, Message};
    use crate::actor_ref::TActorRef;
    use crate::context::{ActorContext, Context};
    use crate::event::event_bus::{EventBusActor, SystemEventBus};
    use crate::event::EventBus;
    use crate::props::Props;
    use crate::provider::ActorRefFactory;
    use crate::system::ActorSystem;
    use crate::system::config::Config;

    #[derive(Debug, EmptyCodec)]
    struct EventMessage1;

    impl Message for EventMessage1 {
        type T = EmptyTestActor;

        fn handle(self: Box<Self>, context: &mut ActorContext, _state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
            info!("{} handle event message {:?}", context.myself(), self);
            Ok(())
        }
    }

    #[derive(Debug, EmptyCodec)]
    struct EventMessage2;

    impl Message for EventMessage2 {
        type T = EmptyTestActor;

        fn handle(self: Box<Self>, context: &mut ActorContext, _state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
            info!("{} handle event message {:?}", context.myself(), self);
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_event_bus() -> anyhow::Result<()> {
        let system = ActorSystem::create(Config::default()).await?;
        let actor1 = system.actor_of(EmptyTestActor, (), Props::default(), Some("actor1".to_string()))?;
        let actor2 = system.actor_of(EmptyTestActor, (), Props::default(), Some("actor2".to_string()))?;
        let actor3 = system.actor_of(EmptyTestActor, (), Props::default(), Some("actor3".to_string()))?;
        let event_bus_actor = system.actor_of(EventBusActor, (), Props::default(), None)?;
        let event_bus = SystemEventBus {
            event_bus: event_bus_actor,
        };
        event_bus.subscribe(actor1.clone(), type_name::<EventMessage1>());
        event_bus.subscribe(actor2.clone(), type_name::<EventMessage1>());
        event_bus.subscribe(actor3.clone(), type_name::<EventMessage1>());
        event_bus.subscribe(actor3.clone(), type_name::<EventMessage2>());
        event_bus.publish(|| DynamicMessage::user(EventMessage1));
        event_bus.publish(|| DynamicMessage::user(EventMessage2));
        event_bus.unsubscribe(actor1, type_name::<EventMessage1>());
        event_bus.publish(|| DynamicMessage::user(EventMessage1));
        event_bus.unsubscribe_all(actor3);
        event_bus.publish(|| DynamicMessage::user(EventMessage2));
        tokio::time::sleep(Duration::from_secs(1)).await;
        actor2.stop();
        tokio::time::sleep(Duration::from_secs(1)).await;
        event_bus.publish(|| DynamicMessage::user(EventMessage1));
        tokio::time::sleep(Duration::from_secs(2)).await;
        Ok(())
    }
}
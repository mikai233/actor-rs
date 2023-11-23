use dashmap::{DashMap, DashSet};
use tracing::debug;

use crate::{CodecMessage, DynamicMessage};
use crate::actor_ref::ActorRef;
use crate::actor_ref::TActorRef;
use crate::event::EventBus;

#[derive(Debug, Default)]
pub struct ActorEventBus {
    subscribers: DashMap<&'static str, DashSet<ActorRef>>,
}

impl EventBus for ActorEventBus {
    type Event = DynamicMessage;
    type Classifier = &'static str;
    type Subscriber = ActorRef;

    fn subscribe(&self, subscriber: Self::Subscriber, to: Self::Classifier) {
        let event_subscribers = self.subscribers.entry(to).or_insert(DashSet::new());
        debug!("subscriber {} subscribe event {}", subscriber, to);
        event_subscribers.insert(subscriber);
    }

    fn unsubscribe(&self, subscriber: &Self::Subscriber, from: Self::Classifier) {
        if let Some(subscribers) = self.subscribers.get(from) {
            subscribers.remove(subscriber);
            debug!("remove subscriber {} from {}", subscriber, from);
        }
    }

    fn unsubscribe_all(&self, subscriber: &Self::Subscriber) {
        self.subscribers.iter().for_each(|value_ref| {
            let event = value_ref.key();
            value_ref.remove(subscriber);
            debug!("remove subscriber {} from {}", subscriber, event);
        });
    }

    fn publish(&self, event_factory: impl Fn() -> Self::Event) {
        let event = event_factory();
        let name = event.name();
        if let Some(value_ref) = self.subscribers.get(name) {
            value_ref.iter().for_each(|subscriber| {
                let subscriber = subscriber.key();
                let event = event_factory();
                subscriber.tell(event, ActorRef::no_sender());
                debug!("publish event {} to subscriber {}", name, subscriber);
            });
        }
    }
}

mod actor_event_bus_test {
    use std::any::type_name;
    use std::time::Duration;

    use tracing::info;

    use actor_derive::EmptyCodec;

    use crate::{Actor, DynamicMessage, EmptyTestActor, Message};
    use crate::context::{ActorContext, Context};
    use crate::event::EventBus;
    use crate::message::MessageRegistration;
    use crate::props::Props;
    use crate::provider::ActorRefFactory;
    use crate::system::ActorSystem;

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
        let system = ActorSystem::new("game".to_string(), "127.0.0.1:12121".parse().unwrap(), MessageRegistration::new())?;
        let actor1 = system.actor_of(EmptyTestActor, (), Props::default(), None)?;
        let actor2 = system.actor_of(EmptyTestActor, (), Props::default(), None)?;
        let actor3 = system.actor_of(EmptyTestActor, (), Props::default(), None)?;
        system.event_bus().subscribe(actor1.clone(), type_name::<EventMessage1>());
        system.event_bus().subscribe(actor2.clone(), type_name::<EventMessage1>());
        system.event_bus().subscribe(actor3.clone(), type_name::<EventMessage1>());
        system.event_bus().subscribe(actor3.clone(), type_name::<EventMessage2>());
        system.event_bus().publish(|| DynamicMessage::user(EventMessage1));
        system.event_bus().publish(|| DynamicMessage::user(EventMessage2));
        tokio::time::sleep(Duration::from_secs(2)).await;
        Ok(())
    }
}
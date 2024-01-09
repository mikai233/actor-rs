use anyhow::anyhow;
use dashmap::{DashMap, DashSet};
use dashmap::mapref::entry::Entry;
use tracing::trace;

use crate::{DynMessage, MessageType};
use crate::actor::actor_ref::ActorRef;
use crate::event::EventBus;

#[derive(Debug, Default)]
pub struct EventStream {
    subscriptions: DashMap<&'static str, DashSet<ActorRef>>,
}

impl EventBus for EventStream {
    type Event = DynMessage;
    type Classifier = &'static str;
    type Subscriber = ActorRef;

    fn subscribe(&self, subscriber: Self::Subscriber, to: Self::Classifier) {
        let subscribers = self.subscriptions.entry(to).or_insert(DashSet::new());
        trace!("{} subscribe to {}", subscriber, to);
        subscribers.insert(subscriber);
    }

    fn unsubscribe(&self, subscriber: Self::Subscriber, from: Self::Classifier) {
        if let Entry::Occupied(mut o) = self.subscriptions.entry(from) {
            o.get_mut().remove(&subscriber);
            trace!("{} unsubscribe from {}", subscriber, from);
        }
    }

    fn unsubscribe_all(&self, subscriber: Self::Subscriber) {
        let mut unsubscribe_events = vec![];
        for event_subscribers in &self.subscriptions {
            if event_subscribers.remove(&subscriber).is_some() {
                unsubscribe_events.push(*event_subscribers.key());
            }
        }
        let event_str = unsubscribe_events.join(",");
        trace!("{} unsubscribe from {}", subscriber, event_str);
    }

    fn publish(&self, event: Self::Event) -> anyhow::Result<()> {
        if matches!(event.message_type, MessageType::Orphan) {
            if event.dyn_clone().is_some() {
                if let Some(subscribers) = self.subscriptions.get(event.name()) {
                    subscribers.iter().for_each(|s| {
                        s.tell(event.dyn_clone().unwrap(), ActorRef::no_sender());
                    });
                }
                Ok(())
            } else {
                Err(anyhow!("event message {} must be cloneable", event.name()))
            }
        } else {
            Err(anyhow!("event message {} must be Orphan type, but is other type", event.name()))
        }
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
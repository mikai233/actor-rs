use dashmap::mapref::entry::Entry;
use dashmap::{DashMap, DashSet};
use tracing::trace;

use crate::actor_ref::{ActorRef, ActorRefExt};
use crate::message::Message;

#[derive(Debug, Default)]
pub struct EventStream {
    subscriptions: DashMap<&'static str, DashSet<ActorRef>>,
}

impl EventStream {
    pub fn subscribe<E>(&self, subscriber: ActorRef)
    where
        E: Message + Clone,
    {
        let signature = E::signature_sized();
        let subscribers = self
            .subscriptions
            .entry(&signature.name)
            .or_insert(DashSet::new());
        trace!("{} subscribe to {}", subscriber, signature);
        subscribers.insert(subscriber);
    }

    pub fn unsubscribe<E>(&self, subscriber: &ActorRef)
    where
        E: Message + Clone,
    {
        let signature = E::signature_sized();
        if let Entry::Occupied(mut o) = self.subscriptions.entry(&signature.name) {
            o.get_mut().retain(|x| x != subscriber);
            trace!("{} unsubscribe from {}", subscriber, signature);
        }
    }

    fn unsubscribe_all(&self, subscriber: &ActorRef) {
        let mut unsubscribe_events = vec![];
        for event_subscribers in &self.subscriptions {
            event_subscribers.retain(|x| {
                if x == subscriber {
                    unsubscribe_events.push(*event_subscribers.key());
                    false
                } else {
                    true
                }
            });
        }
        let events = unsubscribe_events.join(", ");
        trace!("{} unsubscribe from {}", subscriber, events);
    }

    pub fn publish<E>(&self, event: E)
    where
        E: Message + Clone,
    {
        let signature = E::signature_sized();
        if let Some(subscribers) = self.subscriptions.get(&signature.name) {
            subscribers.iter().for_each(move |subscriber| {
                subscriber.cast_ns(event.clone());
            });
        }
    }
}

// #[cfg(test)]
// mod event_tests {
//     use std::time::Duration;

//     use async_trait::async_trait;
//     use tracing::info;

//     use crate::actor::actor_system::ActorSystem;
//     use crate::actor::context::{ActorContext, Context};
//     use crate::actor::props::PropsBuilder;
//     use crate::actor_ref::actor_ref_factory::ActorRefFactory;
//     use crate::config::actor_setting::ActorSetting;
//     use crate::event::event_stream::EventStream;
//     use crate::Message;

//     #[derive(Debug, Clone, Message, derive_more::Display)]
//     #[display("EventMessage")]
//     struct EventMessage;

//     #[async_trait]
//     impl Message for EventWrap {
//         type A = EmptyTestActor;

//         async fn handle(
//             self: Box<Self>,
//             context: &mut Context,
//             _actor: &mut Self::A,
//         ) -> anyhow::Result<()> {
//             info!("{} handle event message {:?}", context.myself(), self);
//             Ok(())
//         }
//     }

//     #[tokio::test]
//     async fn test_event_stream() -> anyhow::Result<()> {
//         let system = ActorSystem::new("mikai233", ActorSetting::default())?;
//         let props_builder = PropsBuilder::new(|()| Ok(EmptyTestActor));
//         let actor1 = system.spawn(props_builder.props(()), "actor1")?;
//         let actor2 = system.spawn(props_builder.props(()), "actor2")?;
//         let actor3 = system.spawn(props_builder.props(()), "actor3")?;
//         let stream = EventStream::default();
//         stream.subscribe::<EventMessage>(actor1.clone());
//         stream.subscribe::<EventMessage>(actor2.clone());
//         stream.subscribe::<EventMessage>(actor3.clone());
//         stream.publish(EventMessage)?;
//         stream.unsubscribe::<EventMessage>(&actor1);
//         stream.publish(EventMessage)?;
//         stream.unsubscribe_all(&actor3);
//         stream.publish(EventMessage)?;
//         tokio::time::sleep(Duration::from_secs(1)).await;
//         actor2.stop();
//         tokio::time::sleep(Duration::from_secs(1)).await;
//         stream.publish(EventMessage)?;
//         tokio::time::sleep(Duration::from_secs(2)).await;
//         Ok(())
//     }
// }

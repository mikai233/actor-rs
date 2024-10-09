use std::any::type_name;

use dashmap::mapref::entry::Entry;
use dashmap::{DashMap, DashSet};
use tracing::{trace, warn};

use crate::actor_ref::ActorRef;
use crate::event::actor_subscriber::ActorSubscriber;
use crate::{CodecMessage, DynMessage};

#[derive(Debug, Default)]
pub struct EventStream {
    subscriptions: DashMap<&'static str, DashSet<ActorSubscriber>>,
}

impl EventStream {
    pub fn subscribe<E, T>(&self, subscriber: ActorRef, transform: T)
        where
            E: CodecMessage,
            T: Fn(E) -> DynMessage + Send + Sync + 'static,
    {
        let event = type_name::<E>();
        let transform = move |message: DynMessage| {
            match message.into_inner().into_any().downcast::<E>() {
                Ok(message) => {
                    Some(transform(*message))
                }
                Err(_) => {
                    None
                }
            }
        };
        let subscriber = ActorSubscriber {
            subscriber,
            transform: Box::new(transform),
        };
        let subscribers = self.subscriptions.entry(event).or_insert(DashSet::new());
        trace!("{} subscribe to {}", subscriber, event);
        subscribers.insert(subscriber);
    }

    pub fn unsubscribe<E>(&self, subscriber: &ActorRef) where E: CodecMessage {
        let event = type_name::<E>();
        if let Entry::Occupied(mut o) = self.subscriptions.entry(event) {
            o.get_mut().retain(|x| { x.subscriber != *subscriber });
            trace!("{} unsubscribe from {}", subscriber, event);
        }
    }

    fn unsubscribe_all(&self, subscriber: &ActorRef) {
        let mut unsubscribe_events = vec![];
        for event_subscribers in &self.subscriptions {
            event_subscribers.retain(|x| {
                if x.subscriber == *subscriber {
                    unsubscribe_events.push(*event_subscribers.key());
                    false
                } else { true }
            });
        }
        let event_str = unsubscribe_events.join(", ");
        trace!("{} unsubscribe from {}", subscriber, event_str);
    }

    pub fn publish<E>(&self, event: E)
    where
        E: Clone,
    {
        let event_name = type_name::<E>();
        if let Some(subscribers) = self.subscriptions.get(event_name) {
            subscribers.iter().for_each(move |s| {
                let msg = (s.transform)(<E as Clone>::clone(&event));
                match msg {
                    None => {
                        warn!("event {} cannot send to {}, transform or message incorrect", event_name, s.subscriber);
                    }
                    Some(msg) => {
                        s.subscriber.tell(msg, ActorRef::no_sender());
                    }
                }
            });
        }
    }
}

#[cfg(test)]
mod event_tests {
    use std::time::Duration;

    use async_trait::async_trait;
    use tracing::info;

    use actor_derive::{COrphanEmptyCodec, EmptyCodec};

    use crate::actor::actor_system::ActorSystem;
    use crate::actor::context::{Context, ActorContext};
    use crate::actor::props::{Props, PropsBuilder};
    use crate::actor_ref::actor_ref_factory::ActorRefFactory;
    use crate::config::actor_setting::ActorSetting;
    use crate::event::event_stream::EventStream;
    use crate::{EmptyTestActor, Message};

    #[derive(Debug, Clone, COrphanEmptyCodec)]
    struct EventMessage;

    #[derive(Debug, EmptyCodec)]
    struct EventWrap(EventMessage);

    #[async_trait]
    impl Message for EventWrap {
        type A = EmptyTestActor;

        async fn handle(self: Box<Self>, context: &mut Context, _actor: &mut Self::A) -> anyhow::Result<()> {
            info!("{} handle event message {:?}", context.myself(), self);
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_event_stream() -> anyhow::Result<()> {
        let system = ActorSystem::new("mikai233", ActorSetting::default())?;
        let props_builder = PropsBuilder::new(|()| { Ok(EmptyTestActor) });
        let actor1 = system.spawn(props_builder.props(()), "actor1")?;
        let actor2 = system.spawn(props_builder.props(()), "actor2")?;
        let actor3 = system.spawn(props_builder.props(()), "actor3")?;
        let stream = EventStream::default();
        stream.subscribe(actor1.clone(), |m| { EventWrap(m).into_dyn() });
        stream.subscribe(actor2.clone(), |m| { EventWrap(m).into_dyn() });
        stream.subscribe(actor3.clone(), |m| { EventWrap(m).into_dyn() });
        stream.publish(EventMessage)?;
        stream.unsubscribe::<EventMessage>(&actor1);
        stream.publish(EventMessage)?;
        stream.unsubscribe_all(&actor3);
        stream.publish(EventMessage)?;
        tokio::time::sleep(Duration::from_secs(1)).await;
        actor2.stop();
        tokio::time::sleep(Duration::from_secs(1)).await;
        stream.publish(EventMessage)?;
        tokio::time::sleep(Duration::from_secs(2)).await;
        Ok(())
    }
}
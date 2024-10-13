pub mod event_stream;
pub mod address_terminated_topic;

pub trait EventBus {
    type Event;

    type Classifier;

    type Subscriber;

    fn subscribe(&self, subscriber: Self::Subscriber, to: Self::Classifier);

    fn unsubscribe(&self, subscriber: &Self::Subscriber, from: Self::Classifier);

    fn unsubscribe_all(&self, subscriber: &Self::Subscriber);

    fn publish(&self, event: Self::Event) -> anyhow::Result<()>;
}

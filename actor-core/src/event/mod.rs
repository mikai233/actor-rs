pub mod event_bus;

pub trait EventBus {
    type Event;
    type Classifier;
    type Subscriber;

    fn subscribe(&self, subscriber: Self::Subscriber, to: Self::Classifier);
    fn unsubscribe(&self, subscriber: Self::Subscriber, from: Self::Classifier);
    fn unsubscribe_all(&self, subscriber: Self::Subscriber);
    fn publish(&self, event_factory: impl Fn() -> Self::Event + Send + 'static);
}
use std::cmp::Ordering;
use std::fmt::{Debug, Display, Formatter};
use std::hash::{Hash, Hasher};

use crate::actor_ref::ActorRef;
use crate::DynMessage;

pub(crate) struct ActorSubscriber {
    pub(crate) subscriber: ActorRef,
    pub(crate) transform: Box<dyn Fn(DynMessage) -> Option<DynMessage> + Send + Sync + 'static>,
}

impl Debug for ActorSubscriber {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ActorSubscriber")
            .field("subscriber", &self.subscriber)
            .finish_non_exhaustive()
    }
}

impl Display for ActorSubscriber {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "ActorSubscriber {}", self.subscriber)
    }
}

impl Hash for ActorSubscriber {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.subscriber.hash(state)
    }
}

impl PartialEq for ActorSubscriber {
    fn eq(&self, other: &Self) -> bool {
        self.subscriber.eq(&other.subscriber)
    }
}

impl Eq for ActorSubscriber {}

impl PartialOrd for ActorSubscriber {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.subscriber.partial_cmp(&other.subscriber)
    }
}

impl Ord for ActorSubscriber {
    fn cmp(&self, other: &Self) -> Ordering {
        self.subscriber.cmp(&other.subscriber)
    }
}
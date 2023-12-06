use crate::Actor;

pub trait IndirectActorProducer: Send + 'static {
    fn produce<T>(&self) -> T where T: Actor;
}


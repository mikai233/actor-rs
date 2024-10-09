use crate::actor_ref::ActorRef;
use crate::provider::TActorRefProvider;

pub struct Provider<P: TActorRefProvider> {
    pub(crate) provider: P,
    pub(crate) actor_refs: Vec<ActorRef>,
}
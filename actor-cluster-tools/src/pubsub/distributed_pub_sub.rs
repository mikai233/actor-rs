use dashmap::mapref::one::MappedRef;

use actor_core::actor::actor_ref::ActorRef;
use actor_core::actor::actor_system::ActorSystem;
use actor_core::actor::extension::Extension;
use actor_core::actor::props::Props;
use actor_derive::AsAny;

use crate::pubsub::distributed_pub_sub_mediator::DistributedPubSubMediator;

#[derive(Debug, AsAny)]
pub struct DistributedPubSub {
    system: ActorSystem,
    mediator: ActorRef,
}

impl DistributedPubSub {
    pub fn new(system: ActorSystem) -> anyhow::Result<Self> {
        let mediator = system.spawn_system(
            Props::new(|| Ok(DistributedPubSubMediator {})),
            Some("distributed_pub_sub_mediator".to_string()),
        )?;
        let myself = Self {
            system,
            mediator,
        };
        Ok(myself)
    }

    pub fn get(system: &ActorSystem) -> MappedRef<&'static str, Box<dyn Extension>, Self> {
        system.get_extension::<Self>().expect("DistributedPubSub extension not found")
    }
}
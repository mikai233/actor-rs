use std::any::type_name;

use actor_core::actor::actor_system::{ActorSystem, WeakSystem};
use actor_core::actor::extension::Extension;
use actor_core::actor::props::Props;
use actor_core::actor_ref::ActorRef;
use actor_core::AsAny;

use crate::pubsub::distributed_pub_sub_mediator::DistributedPubSubMediator;

#[derive(Debug, Clone, AsAny)]
pub struct DistributedPubSub {
    system: WeakSystem,
    mediator: ActorRef,
}

impl DistributedPubSub {
    pub fn new(system: ActorSystem) -> anyhow::Result<Self> {
        let mediator = system.spawn_system(
            Props::new(|| Ok(DistributedPubSubMediator {})),
            Some("distributed_pub_sub_mediator".to_string()),
        )?;
        let myself = Self {
            system: system.downgrade(),
            mediator,
        };
        Ok(myself)
    }

    pub fn get(system: &ActorSystem) -> Self {
        system.get_extension::<Self>().expect(&format!("{} not found", type_name::<Self>()))
    }
}

impl Extension for DistributedPubSub {}
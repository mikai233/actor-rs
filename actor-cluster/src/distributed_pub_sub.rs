use async_trait::async_trait;
use dashmap::mapref::one::MappedRef;
use tracing::trace;

use actor_core::Actor;
use actor_core::actor::actor_ref::ActorRef;
use actor_core::actor::actor_system::ActorSystem;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::extension::Extension;
use actor_core::actor::props::Props;
use actor_derive::AsAny;

#[derive(Debug, AsAny)]
pub struct DistributedPubSub {
    system: ActorSystem,
    mediator: ActorRef,
}

impl DistributedPubSub {
    pub fn new(system: ActorSystem) -> Self {
        let mediator = system.spawn_system_actor(
            Props::create(|_| DistributedPubSubMediator {}),
            Some("distributed_pub_sub_mediator".to_string()),
        ).expect("Failed to create DistributedPubSubMediator");
        Self {
            system,
            mediator,
        }
    }

    pub fn get(system: &ActorSystem) -> MappedRef<&str, Box<dyn Extension + '_>, Self> {
        system.get_extension::<Self>().expect("DistributedPubSub extension not found")
    }
}

#[derive(Debug)]
struct DistributedPubSubMediator {}

#[async_trait]
impl Actor for DistributedPubSubMediator {
    async fn pre_start(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        trace!("{} started", context.myself());
        Ok(())
    }
}
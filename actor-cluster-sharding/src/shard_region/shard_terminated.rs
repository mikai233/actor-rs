use async_trait::async_trait;
use tracing::debug;

use actor_core::actor::context::ActorContext;
use actor_core::actor_ref::ActorRef;
use actor_core::Message;
use actor_core::message::terminated::Terminated;
use actor_derive::EmptyCodec;

use crate::shard_region::ShardRegion;

#[derive(Debug, EmptyCodec)]
pub(super) struct ShardTerminated(pub(super) ActorRef);

impl Terminated for ShardTerminated {
    fn actor(&self) -> &ActorRef {
        &self.0
    }
}

#[async_trait]
impl Message for ShardTerminated {
    type A = ShardRegion;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let actor_ref = self.0;
        if actor.coordinator.as_ref().is_some_and(|coordinator| coordinator == &actor_ref) {
            actor.coordinator = None;
            actor.start_registration(context)?;
        } else if actor.regions.contains_key(&actor_ref) {
            if let Some(shards) = actor.regions.remove(&actor_ref) {
                for shard in &shards {
                    actor.region_by_shard.remove(shard);
                }
                let type_name = &actor.type_name;
                let size = shards.len();
                let shard_str = shards.into_iter().collect::<Vec<_>>().join(", ");
                debug!("{type_name}: Region [{actor_ref}] terminated with [{size}] shards [{shard_str}]");
            }
        } else if actor.shards_by_ref.contains_key(&actor_ref) {
            if let Some(shard_id) = actor.shards_by_ref.remove(&actor_ref) {
                actor.shards.remove(&shard_id);
                actor.starting_shards.remove(&shard_id);
                //TODO passivation strategy
                let type_name = &actor.type_name;
                match actor.handing_off.remove(&actor_ref) {
                    true => {
                        debug!("{type_name}: Shard [{shard_id}] handoff complete")
                    }
                    false => {
                        debug!("{type_name}: Shard [{shard_id}] terminated while not being handed off");
                    }
                }
            }
        }
        Ok(())
    }
}
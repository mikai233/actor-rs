use actor_core::actor_ref::ActorRef;
use actor_core::OrphanEmptyCodec;

#[derive(Debug, OrphanEmptyCodec)]
pub(crate) struct Started {
    pub(crate) shard_region: ActorRef,
}
use actor_core::OrphanEmptyCodec;
use actor_core::actor_ref::ActorRef;

#[derive(Debug, OrphanEmptyCodec)]
pub(crate) struct Started {
    pub(crate) shard_region: ActorRef,
}

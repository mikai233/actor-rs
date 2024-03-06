use actor_core::actor_ref::ActorRef;
use actor_derive::OrphanEmptyCodec;

#[derive(Debug, OrphanEmptyCodec)]
pub(crate) struct Started {
    pub(crate) shard_region: ActorRef,
}
use kairo_core::OrphanEmptyCodec;
use kairo_core::actor_ref::ActorRef;

#[derive(Debug, OrphanEmptyCodec)]
pub(crate) struct Started {
    pub(crate) shard_region: ActorRef,
}

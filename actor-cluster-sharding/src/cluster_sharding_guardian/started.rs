use actor_core::actor_ref::ActorRef;
use actor_core::Message;

#[derive(Debug, Message, derive_more::Display, derive_more::Constructor)]
#[display("Started {{ shard_region: {shard_region} }}")]
pub(crate) struct Started {
    pub(crate) shard_region: ActorRef,
}
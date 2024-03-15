use actor_core::actor_ref::ActorRef;

#[derive(Debug)]
pub(super) enum DeliverTarget<'a> {
    Shard(&'a ActorRef),
    ShardRegion(&'a ActorRef),
}
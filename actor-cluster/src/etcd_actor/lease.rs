use actor_core::actor_ref::ActorRef;

#[derive(Debug)]
pub(super) struct Lease {
    pub(super) keeper: ActorRef,
    pub(super) stream: etcd_client::LeaseKeepAliveStream,
    pub(super) watcher: ActorRef,
}
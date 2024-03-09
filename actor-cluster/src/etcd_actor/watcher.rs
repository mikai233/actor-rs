use actor_core::actor_ref::ActorRef;

#[derive(Debug)]
pub(super) struct Watcher {
    pub(super) watcher: etcd_client::Watcher,
    pub(super) stream: etcd_client::WatchStream,
    pub(super) applicant: ActorRef,
}
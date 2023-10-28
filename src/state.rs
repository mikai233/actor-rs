#[derive(Debug)]
pub(crate) enum ActorState {
    Init,
    Started,
    Stopping,
    WaitingChildrenStop,
    Stopped,
}
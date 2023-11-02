#[derive(Debug)]
pub(crate) enum ActorState {
    Init,
    Started,
    Terminating,
    WaitingChildrenStop,
    Terminated,
}
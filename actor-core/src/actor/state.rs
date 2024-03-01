#[derive(Debug)]
pub(crate) enum ActorState {
    Init,
    Started,
    Suspend,
    CanTerminate,
    Terminating,
    Terminated,
}
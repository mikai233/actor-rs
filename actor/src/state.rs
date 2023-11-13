#[derive(Debug)]
pub(crate) enum ActorState {
    Init,
    Started,
    CanTerminate,
    Terminating,
    Terminated,
}
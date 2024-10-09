use std::fmt::{Display, Formatter};

#[derive(Debug, Copy, Clone)]
pub(crate) enum ActorState {
    Init,
    Started,
    Suspend,
    CanTerminate,
    Terminating,
    Terminated,
}

impl Display for ActorState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ActorState::Init => {
                write!(f, "Init")
            }
            ActorState::Started => {
                write!(f, "Started")
            }
            ActorState::Suspend => {
                write!(f, "Suspend")
            }
            ActorState::CanTerminate => {
                write!(f, "CanTerminate")
            }
            ActorState::Terminating => {
                write!(f, "Terminating")
            }
            ActorState::Terminated => {
                write!(f, "Terminated")
            }
        }
    }
}
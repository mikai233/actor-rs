use std::fmt::{Display, Formatter};

use actor_core::actor_ref::ActorRef;

#[derive(Debug)]
pub(super) enum EntityState {
    NoState,
    Active(ActorRef),
    Passivation(ActorRef),
    WaitingForRestart,
}

impl Display for EntityState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            EntityState::NoState => {
                write!(f, "NoState")
            }
            EntityState::Active(_) => {
                write!(f, "Active")
            }
            EntityState::Passivation(_) => {
                write!(f, "Passivation")
            }
            EntityState::WaitingForRestart => {
                write!(f, "WaitingForRestart")
            }
        }
    }
}
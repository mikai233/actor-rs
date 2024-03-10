use std::fmt::{Display, Formatter};

use actor_core::actor_ref::ActorRef;

use crate::shard_region::ImEntityId;

#[derive(Debug)]
pub(super) enum EntityState {
    NoState,
    Active(ImEntityId, ActorRef),
    Passivation(ImEntityId, ActorRef),
    WaitingForRestart,
}

impl Display for EntityState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            EntityState::NoState => {
                write!(f, "NoState")
            }
            EntityState::Active(_, entity) => {
                write!(f, "Active {}", entity)
            }
            EntityState::Passivation(_, entity) => {
                write!(f, "Passivation {}", entity)
            }
            EntityState::WaitingForRestart => {
                write!(f, "WaitingForRestart")
            }
        }
    }
}
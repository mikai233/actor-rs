use std::fmt::{Debug, Display, Formatter};

use enum_dispatch::enum_dispatch;
use itertools::Itertools;

use crate::actor_ref::ActorRef;
use crate::message::DynMessage;
use crate::routing::routee::actor_ref_routee::ActorRefRoutee;
use crate::routing::routee::actor_selection_routee::ActorSelectionRoutee;
use crate::routing::routee::no_routee::NoRoutee;
use crate::routing::routee::several_routees::SeveralRoutees;

pub mod actor_ref_routee;
pub mod actor_selection_routee;
pub mod no_routee;
pub mod several_routees;

#[enum_dispatch(Routee)]
pub trait TRoutee {
    fn send(&self, message: DynMessage, sender: Option<ActorRef>);
}

#[enum_dispatch]
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum Routee {
    ActorRefRoutee,
    ActorSelectionRoutee,
    NoRoutee,
    SeveralRoutees,
}

impl Display for Routee {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Routee::ActorRefRoutee(routee) => {
                write!(f, "ActorRefRoutee({})", routee.0)
            }
            Routee::ActorSelectionRoutee(routee) => {
                write!(f, "ActorSelectionRoutee({})", routee.0)
            }
            Routee::NoRoutee(_) => {
                write!(f, "NoRoutee")
            }
            Routee::SeveralRoutees(routee) => {
                write!(f, "SeveralRoutees({})", routee.routees.iter().join(", "))
            }
        }
    }
}

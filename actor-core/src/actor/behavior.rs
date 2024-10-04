use crate::actor::receive::Receive;
use crate::actor::Actor;

pub enum Behavior<A: Actor> {
    Same,
    Become {
        receive: Receive<A>,
        discard_old: bool,
    },
    Unbecome,
}

impl<A: Actor> Behavior<A> {
    pub fn same() -> Self {
        Self::Same
    }

    pub fn r#become(receive: Receive<A>, discard_old: bool) -> Self {
        Self::Become { receive, discard_old }
    }

    pub fn unbecome() -> Self {
        Self::Unbecome
    }
}
use crate::actor::receive::Receive;
use crate::actor::Actor;
use crate::cell::actor_cell::ActorCell;

#[derive(Debug)]
pub(crate) struct UserGuardian;


impl Actor for UserGuardian {
    type Context = ActorCell;

    fn receive(&self) -> Receive<Self> {
        todo!()
    }
}

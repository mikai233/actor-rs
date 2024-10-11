use crate::actor::receive::Receive;
use crate::actor::Actor;
use crate::message::stop_child::StopChild;

use super::context::Context;

#[derive(Debug)]
pub(crate) struct UserGuardian;

impl Actor for UserGuardian {
    type Context = Context;

    fn receive(&self) -> Receive<Self> {
        Receive::new().handle::<StopChild>()
    }
}

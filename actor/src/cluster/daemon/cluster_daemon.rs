use crate::Actor;
use crate::cluster::daemon::state::State;
use crate::context::ActorContext;

#[derive(Debug)]
pub(crate) struct ClusterDaemon;

impl Actor for ClusterDaemon {
    type S = State;
    type A = ();

    fn pre_start(&self, _context: &mut ActorContext, _arg: Self::A) -> anyhow::Result<Self::S> {
        todo!()
    }
}
use crate::actor::Actor;
use crate::actor::context::ActorContext;
use crate::cluster::daemon::state::State;

#[derive(Debug)]
pub(crate) struct ClusterDaemon;

impl Actor for ClusterDaemon {
    type S = State;
    type A = ();

    fn pre_start(&self, context: &mut ActorContext, arg: Self::A) -> anyhow::Result<Self::S> {
        todo!()
    }
}
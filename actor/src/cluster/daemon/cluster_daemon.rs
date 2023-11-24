use async_trait::async_trait;
use crate::Actor;
use crate::cluster::daemon::state::State;
use crate::context::ActorContext;

#[derive(Debug)]
pub(crate) struct ClusterDaemon;

#[async_trait]
impl Actor for ClusterDaemon {
    type S = State;
    type A = ();

    async fn pre_start(&self, _context: &mut ActorContext, _arg: Self::A) -> anyhow::Result<Self::S> {
        todo!()
    }
}
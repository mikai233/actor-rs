use actor_core::actor::context::Context;
use actor_core::actor::Actor;
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::actor_ref::ActorRef;

use crate::cluster::Cluster;
use crate::coordinated_shutdown_leave::cluster_event::ClusterEventWrap;
use crate::coordinated_shutdown_leave::leave_resp::LeaveResp;

mod cluster_event;
pub(crate) mod leave_resp;

#[derive(Debug)]
pub(crate) struct CoordinatedShutdownLeave {
    cluster: Cluster,
    reply_to: ActorRef,
}

impl CoordinatedShutdownLeave {
    pub(crate) fn new(context: &mut Context, reply_to: ActorRef) -> Self {
        let cluster = Cluster::get(context.system()).clone();
        Self { cluster, reply_to }
    }

    fn done(&self, context: &mut Context) {
        self.reply_to.cast_orphan_ns(LeaveResp);
        context.stop(context.myself());
    }
}

impl Actor for CoordinatedShutdownLeave {
    type Context = Context;

    fn started(&mut self, ctx: &mut Self::Context) -> anyhow::Result<()> {
        self.cluster.subscribe(ctx.myself().clone(), |event| {
            ClusterEventWrap(event).into_dyn()
        })?;
        self.cluster.leave(self.cluster.self_address().clone());
        Ok(())
    }

    fn stopped(&mut self, ctx: &mut Self::Context) -> anyhow::Result<()> {
        self.cluster.unsubscribe_cluster_event(ctx.myself())?;
        Ok(())
    }

    fn receive(&self) -> actor_core::actor::receive::Receive<Self> {
        todo!()
    }
}

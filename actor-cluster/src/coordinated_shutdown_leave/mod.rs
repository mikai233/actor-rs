use async_trait::async_trait;

use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::actor_ref::{ActorRef, ActorRefExt};

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
        Self {
            cluster,
            reply_to,
        }
    }

    fn done(&self, context: &mut Context) {
        self.reply_to.cast_orphan_ns(LeaveResp);
        context.stop(context.myself());
    }
}

#[async_trait]
impl Actor for CoordinatedShutdownLeave {
    async fn started(&mut self, context: &mut Context) -> anyhow::Result<()> {
        self.cluster.subscribe(
            context.myself().clone(),
            |event| { ClusterEventWrap(event).into_dyn() },
        )?;
        self.cluster.leave(self.cluster.self_address().clone());
        Ok(())
    }

    async fn stopped(&mut self, context: &mut Context) -> anyhow::Result<()> {
        self.cluster.unsubscribe_cluster_event(context.myself())?;
        Ok(())
    }

    async fn on_recv(&mut self, context: &mut Context, message: DynMessage) -> anyhow::Result<()> {
        Self::handle_message(self, context, message).await
    }
}
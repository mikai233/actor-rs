use async_trait::async_trait;

use actor_core::Actor;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor_ref::{ActorRef, ActorRefExt};
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::ext::message_ext::UserMessageExt;

use crate::cluster::Cluster;
use crate::coordinated_shutdown_leave::cluster_event_wrap::ClusterEventWrap;
use crate::coordinated_shutdown_leave::leave_resp::LeaveResp;

mod cluster_event_wrap;
pub(crate) mod leave_resp;

#[derive(Debug)]
pub(crate) struct CoordinatedShutdownLeave {
    cluster: Cluster,
    reply_to: ActorRef,
}

impl CoordinatedShutdownLeave {
    pub(crate) fn new(context: &mut ActorContext, reply_to: ActorRef) -> Self {
        let cluster = Cluster::get(context.system()).clone();
        cluster.leave(cluster.self_address().clone());
        Self {
            cluster,
            reply_to,
        }
    }

    fn done(&self, context: &mut ActorContext) {
        self.reply_to.resp(LeaveResp);
        context.stop(context.myself());
    }
}

#[async_trait]
impl Actor for CoordinatedShutdownLeave {
    async fn started(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        self.cluster.subscribe_cluster_event(
            context.myself().clone(),
            |event| { ClusterEventWrap(event).into_dyn() },
        )?;
        Ok(())
    }

    async fn stopped(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        self.cluster.unsubscribe_cluster_event(context.myself())?;
        Ok(())
    }
}
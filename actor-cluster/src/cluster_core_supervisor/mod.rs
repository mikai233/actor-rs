use anyhow::Error;
use async_trait::async_trait;

use actor_core::Actor;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::directive::Directive;
use actor_core::actor::props::Props;
use actor_core::actor_ref::{ActorRef, ActorRefSystemExt};
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::message::poison_pill::PoisonPill;

use crate::cluster::Cluster;
use crate::cluster_core_daemon::ClusterCoreDaemon;
use crate::cluster_core_supervisor::core_daemon_terminated::CoreDaemonTerminated;

pub(crate) mod get_cluster_core_ref;
mod core_daemon_terminated;

#[derive(Debug)]
pub(crate) struct ClusterCoreSupervisor {
    core_daemon: Option<ActorRef>,
    cluster: Cluster,
}

impl ClusterCoreSupervisor {
    pub(crate) fn new(context: &mut ActorContext) -> Self {
        let cluster = Cluster::get(context.system()).clone();
        Self {
            core_daemon: None,
            cluster,
        }
    }

    fn create_children(&mut self, context: &mut ActorContext) -> anyhow::Result<ActorRef> {
        let core_daemon = context.spawn(
            Props::new_with_ctx(|ctx| ClusterCoreDaemon::new(ctx)),
            "daemon",
        )?;
        context.watch(CoreDaemonTerminated(core_daemon.clone()));
        self.core_daemon = Some(core_daemon.clone());
        Ok(core_daemon)
    }
}

#[async_trait]
impl Actor for ClusterCoreSupervisor {
    async fn stopped(&mut self, _context: &mut ActorContext) -> anyhow::Result<()> {
        self.cluster.shutdown()?;
        Ok(())
    }

    fn on_child_failure(&mut self, context: &mut ActorContext, child: &ActorRef, error: &Error) -> Directive {
        //TODO check panic error
        context.myself().cast_system(PoisonPill, ActorRef::no_sender());
        Directive::Stop
    }
}
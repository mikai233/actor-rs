use actor_core::actor::Actor;
use anyhow::Error;

use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::directive::Directive;
use actor_core::actor::props::Props;
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::actor_ref::ActorRef;
use actor_core::message::poison_pill::PoisonPill;

use crate::cluster::Cluster;
use crate::cluster_core_daemon::ClusterCoreDaemon;
use crate::cluster_core_supervisor::core_daemon_terminated::CoreDaemonTerminated;

mod core_daemon_terminated;
pub(crate) mod get_cluster_core_ref;

#[derive(Debug)]
pub(crate) struct ClusterCoreSupervisor {
    core_daemon: Option<ActorRef>,
    cluster: Cluster,
}

impl ClusterCoreSupervisor {
    pub(crate) fn new(context: &mut Context) -> Self {
        let cluster = Cluster::get(context.system()).clone();
        Self {
            core_daemon: None,
            cluster,
        }
    }

    fn create_children(&mut self, context: &mut Context) -> anyhow::Result<ActorRef> {
        let core_daemon = context.spawn(
            Props::new_with_ctx(|ctx| ClusterCoreDaemon::new(ctx)),
            "daemon",
        )?;
        context.watch_with(core_daemon.clone(), CoreDaemonTerminated::new)?;
        self.core_daemon = Some(core_daemon.clone());
        Ok(core_daemon)
    }
}

impl Actor for ClusterCoreSupervisor {
    type Context = Context;
    fn stopped(&mut self, ctx: &mut Self::Context) -> anyhow::Result<()> {
        self.cluster.shutdown()?;
        Ok(())
    }

    fn on_child_failure(
        &mut self,
        context: &mut Self::Context,
        child: &ActorRef,
        error: &anyhow::Error,
    ) -> Directive {
        //TODO check panic error
        context
            .myself()
            .cast_system(PoisonPill, ActorRef::no_sender());
        Directive::Stop
    }

    fn receive(&self) -> actor_core::actor::receive::Receive<Self> {
        todo!()
    }
}

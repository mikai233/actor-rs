use actor_core::actor::Actor;
use anyhow::Error;

use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::directive::Directive;
use actor_core::actor::props::Props;
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::actor_ref::{ActorRef, ActorRefExt};
use actor_core::message::poison_pill::PoisonPill;

use crate::cluster::Cluster;
use crate::cluster_core_daemon::ClusterCoreDaemon;

mod core_daemon_terminated;
pub(crate) mod get_cluster_core_ref;

#[derive(Debug)]
pub(crate) struct ClusterCoreSupervisor {
    core_daemon: Option<ActorRef>,
    cluster: Cluster,
}

impl ClusterCoreSupervisor {
    pub(crate) fn new(context: &mut <Self as Actor>::Context) -> Self {
        let cluster = Cluster::get(context.system()).clone();
        Self {
            core_daemon: None,
            cluster,
        }
    }

    fn create_children(&mut self, ctx: &mut <Self as Actor>::Context) -> anyhow::Result<ActorRef> {
        //TODO publisher
        let core_daemon = ctx.spawn(
            Props::new_with_ctx(|ctx| ClusterCoreDaemon::new(ctx)),
            "daemon",
        )?;
        ctx.watch(&core_daemon)?;
        self.core_daemon = Some(core_daemon);
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
        ctx: &mut Self::Context,
        child: &ActorRef,
        error: &anyhow::Error,
    ) -> Directive {
        //TODO check panic error
        ctx.myself().cast_ns(PoisonPill);
        Directive::Stop
    }

    fn receive(&self) -> actor_core::actor::receive::Receive<Self> {
        todo!()
    }
}

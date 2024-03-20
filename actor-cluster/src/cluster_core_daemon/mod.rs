use std::ops::Not;

use anyhow::Context as AnyhowContext;
use async_trait::async_trait;
use tokio::sync::mpsc::{channel, Sender};
use tracing::debug;

use actor_core::Actor;
use actor_core::actor::actor_selection::{ActorSelection, ActorSelectionPath};
use actor_core::actor::address::Address;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::coordinated_shutdown::{CoordinatedShutdown, PHASE_CLUSTER_EXITING, PHASE_CLUSTER_EXITING_DONE};
use actor_core::actor_path::root_actor_path::RootActorPath;
use actor_core::actor_path::TActorPath;
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::ext::option_ext::OptionExt;
use actor_core::ext::type_name_of;
use actor_core::pattern::patterns::PatternsExt;

use crate::cluster::Cluster;
use crate::cluster_core_daemon::exiting_completed_req::{ExitingCompletedReq, ExitingCompletedResp};
use crate::member::MemberStatus;

mod exiting_completed_req;

#[derive(Debug)]
pub(crate) struct ClusterCoreDaemon {
    self_exiting: Sender<()>,
}

impl ClusterCoreDaemon {
    pub(crate) fn new(context: &mut ActorContext) -> anyhow::Result<Self> {
        let (self_exiting_tx, mut self_exiting_rx) = channel(1);
        let coord_shutdown = CoordinatedShutdown::get(context.system());
        let cluster_ext = Cluster::get(context.system()).clone();
        let cluster = cluster_ext.clone();
        coord_shutdown.add_task(PHASE_CLUSTER_EXITING, "wait-exiting", async move {
            if cluster.members().is_empty().not() {
                self_exiting_rx.recv().await;
            }
        })?;
        let myself = context.myself().clone();
        let cluster = cluster_ext.clone();
        let phase_cluster_exiting_done_timeout = coord_shutdown.timeout(PHASE_CLUSTER_EXITING_DONE)
            .into_result()
            .context(format!("phase {} not found", PHASE_CLUSTER_EXITING_DONE))?;
        coord_shutdown.add_task(PHASE_CLUSTER_EXITING_DONE, "exiting-completed", async move {
            if !(cluster.is_terminated() || cluster.self_member().status == MemberStatus::Removed) {
                if let Some(error) = myself.ask::<_, ExitingCompletedResp>(ExitingCompletedReq, phase_cluster_exiting_done_timeout).await.err() {
                    debug!("ask {} error {:?}", type_name_of::<ExitingCompletedResp>(), error);
                }
            }
        })?;
        let daemon = Self {
            self_exiting: self_exiting_tx,
        };
        Ok(daemon)
    }

    fn cluster_core(context: &mut ActorContext, address: Address) -> anyhow::Result<ActorSelection> {
        let path = RootActorPath::new(address, "/")
            .child("system")
            .child("cluster")
            .child("core")
            .child("daemon");
        context.actor_selection(ActorSelectionPath::FullPath(path))
    }
}

#[async_trait]
impl Actor for ClusterCoreDaemon {
    async fn stopped(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        let _ = self.self_exiting.send(()).await;
        Ok(())
    }
}
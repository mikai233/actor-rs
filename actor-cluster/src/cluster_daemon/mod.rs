use std::any::type_name;
use std::fmt::Debug;

use anyhow::Context as _;
use async_trait::async_trait;
use tokio::sync::mpsc::{channel, Sender};
use tracing::{debug, warn};

use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::coordinated_shutdown::{ClusterDowningReason, CoordinatedShutdown, PHASE_CLUSTER_LEAVE, PHASE_CLUSTER_SHUTDOWN};
use actor_core::actor::props::Props;
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::actor_ref::ActorRef;
use actor_core::ext::option_ext::OptionExt;
use actor_core::pattern::patterns::PatternsExt;
use actor_core::{Actor, DynMessage};

use crate::cluster::Cluster;
use crate::cluster_core_supervisor::ClusterCoreSupervisor;
use crate::cluster_daemon::leave_req::LeaveReq;
use crate::coordinated_shutdown_leave::leave_resp::LeaveResp;
use crate::heartbeat::cluster_heartbeat_receiver::ClusterHeartbeatReceiver;
use crate::member::MemberStatus;

pub(crate) mod add_on_member_up_listener;
pub(crate) mod add_on_member_removed_listener;
mod leave_req;
pub(crate) mod get_cluster_core_ref_req;

trait ClusterMessage {}

pub(crate) mod cluster_user_action {
    use async_trait::async_trait;

    use actor_core::actor::address::Address;
    use actor_core::actor::context::ActorContext;
    use actor_core::util::version::Version;
    use actor_core::{EmptyCodec, Message};

    use crate::cluster_core_daemon::ClusterCoreDaemon;
    use crate::cluster_daemon::ClusterMessage;

    #[derive(Debug, Clone, EmptyCodec)]
    pub(crate) struct JoinTo {
        pub(crate) address: Address,
    }

    #[async_trait]
    impl Message for JoinTo {
        type A = ClusterCoreDaemon;

        async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
            todo!()
        }
    }

    #[derive(Debug, Clone, EmptyCodec)]
    pub(crate) struct Leave {
        pub(crate) address: Address,
    }

    #[async_trait]
    impl Message for Leave {
        type A = ClusterCoreDaemon;

        async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
            todo!()
        }
    }

    impl ClusterMessage for Leave {}

    #[derive(Debug, Clone, EmptyCodec)]
    pub(crate) struct Down {
        pub(crate) address: Address,
    }

    impl ClusterMessage for Down {}

    #[async_trait]
    impl Message for Down {
        type A = ClusterCoreDaemon;

        async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
            todo!()
        }
    }

    #[derive(Debug, Clone, EmptyCodec)]
    pub(crate) struct PrepareForShutdown;

    impl ClusterMessage for PrepareForShutdown {}

    #[async_trait]
    impl Message for PrepareForShutdown {
        type A = ClusterCoreDaemon;

        async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
            todo!()
        }
    }

    #[derive(Debug, Clone, EmptyCodec)]
    pub(crate) struct SetAppVersionLater;

    #[async_trait]
    impl Message for SetAppVersionLater {
        type A = ClusterCoreDaemon;

        async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
            todo!()
        }
    }

    #[derive(Debug, Clone, EmptyCodec)]
    pub(crate) struct SetAppVersion {
        pub(crate) app_version: Version,
    }

    #[async_trait]
    impl Message for SetAppVersion {
        type A = ClusterCoreDaemon;

        async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
            todo!()
        }
    }
}

#[derive(Debug)]
pub struct ClusterDaemon {
    core_supervisor: Option<ActorRef>,
    cluster_shutdown: Sender<()>,
}

#[async_trait]
impl Actor for ClusterDaemon {
    async fn started(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        let cluster = Cluster::get(context.system()).clone();
        let myself = context.myself().clone();
        let cluster_shutdown = self.cluster_shutdown.clone();
        let coord_shutdown = CoordinatedShutdown::get(context.system());
        let phase_cluster_leave_timeout = CoordinatedShutdown::timeout(context.system(), PHASE_CLUSTER_LEAVE)
            .into_result()
            .context(format!("phase {} not found", PHASE_CLUSTER_LEAVE))?;
        coord_shutdown.add_task(context.system(), PHASE_CLUSTER_LEAVE, "leave", async move {
            if cluster.is_terminated() || cluster.self_member().status == MemberStatus::Removed {
                if let Some(_) = cluster_shutdown.send(()).await.err() {
                    debug!("send shutdown failed because receiver already closed");
                }
            } else {
                if let Some(error) = myself.ask::<_, LeaveResp>(LeaveReq, phase_cluster_leave_timeout).await.err() {
                    warn!("ask {} error {:?}", type_name::<LeaveReq>(), error);
                }
            }
        })?;
        Ok(())
    }

    async fn stopped(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        let _ = self.cluster_shutdown.send(()).await;
        let system = context.system().clone();
        let fut = {
            let coord_shutdown = CoordinatedShutdown::get(&system);
            coord_shutdown.run(ClusterDowningReason)
        };
        tokio::spawn(fut);
        Ok(())
    }

    async fn on_recv(&mut self, context: &mut ActorContext, message: DynMessage) -> anyhow::Result<()> {
        Self::handle_message(self, context, message).await
    }
}

impl ClusterDaemon {
    pub(crate) fn new(context: &mut ActorContext) -> anyhow::Result<Self> {
        let coord_shutdown = CoordinatedShutdown::get(context.system());
        let (cluster_shutdown_tx, mut cluster_shutdown_rx) = channel(1);
        coord_shutdown.add_task(context.system(), PHASE_CLUSTER_SHUTDOWN, "wait-shutdown", async move {
            let _ = cluster_shutdown_rx.recv().await;
        })?;
        let daemon = Self {
            core_supervisor: None,
            cluster_shutdown: cluster_shutdown_tx,
        };
        Ok(daemon)
    }

    fn create_children(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        let core_supervisor = context.spawn(
            Props::new_with_ctx(|ctx| {
                Ok(ClusterCoreSupervisor::new(ctx))
            }),
            "core",
        )?;
        self.core_supervisor = Some(core_supervisor);
        context.spawn(ClusterHeartbeatReceiver::props(), ClusterHeartbeatReceiver::name())?;
        Ok(())
    }
}

pub(crate) mod internal_cluster_action {
    use ahash::HashSet;
    use async_trait::async_trait;
    use imstr::ImString;

    use actor_core::actor::address::Address;
    use actor_core::actor::context::ActorContext;
    use actor_core::actor_ref::ActorRef;
    use actor_core::util::version::Version;
    use actor_core::{DynMessage, EmptyCodec, Message};

    use crate::cluster_daemon::{ClusterDaemon, ClusterMessage};
    use crate::cluster_event::{ClusterDomainEvent, SubscriptionInitialStateMode};
    use crate::gossip::Gossip;
    use crate::unique_address::UniqueAddress;

    #[derive(Debug, EmptyCodec)]
    pub(crate) struct Join {
        pub(crate) node: UniqueAddress,
        pub(crate) roles: HashSet<ImString>,
        pub(crate) app_version: Version,
    }

    impl ClusterMessage for Join {}

    #[async_trait]
    impl Message for Join {
        type A = ClusterDaemon;

        async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
            todo!()
        }
    }

    #[derive(Debug, EmptyCodec)]
    pub(crate) struct Welcome {
        pub(crate) from: UniqueAddress,
        pub(crate) gossip: Gossip,
    }

    impl ClusterMessage for Welcome {}

    #[async_trait]
    impl Message for Welcome {
        type A = ClusterDaemon;

        async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
            todo!()
        }
    }

    #[derive(Debug, EmptyCodec)]
    pub(crate) struct JoinSeedNodes {
        pub(crate) seed_nodes: Vec<Address>,
    }

    #[async_trait]
    impl Message for JoinSeedNodes {
        type A = ClusterDaemon;

        async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
            todo!()
        }
    }

    #[derive(Debug, EmptyCodec)]
    pub(crate) struct JoinSeedNode;

    #[async_trait]
    impl Message for JoinSeedNode {
        type A = ClusterDaemon;

        async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
            todo!()
        }
    }

    #[derive(Debug, EmptyCodec)]
    pub struct Subscribe {
        pub subscriber: ActorRef,
        pub initial_state_mode: SubscriptionInitialStateMode,
        pub to: &'static str,
        pub transform: Box<dyn Fn(Box<dyn ClusterDomainEvent>) -> DynMessage>,
    }

    impl Message for Subscribe {
        type A = ();

        async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
            todo!()
        }
    }

    #[derive(Debug, EmptyCodec)]
    pub struct Unsubscribe {
        pub subscriber: ActorRef,
        pub to: Option<&'static str>,
    }
}
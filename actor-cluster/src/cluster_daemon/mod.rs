use std::any::type_name;
use std::fmt::Debug;

use actor_core::actor::Actor;
use anyhow::Context as _;
use tokio::sync::mpsc::{channel, Sender};
use tracing::{debug, warn};

use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::coordinated_shutdown::{
    ClusterDowningReason, CoordinatedShutdown, PHASE_CLUSTER_LEAVE, PHASE_CLUSTER_SHUTDOWN,
};
use actor_core::actor::props::Props;
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::actor_ref::ActorRef;
use actor_core::pattern::patterns::PatternsExt;

use crate::cluster::Cluster;
use crate::cluster_core_supervisor::ClusterCoreSupervisor;
use crate::cluster_daemon::leave_req::LeaveReq;
use crate::coordinated_shutdown_leave::leave_resp::LeaveResp;
use crate::heartbeat::cluster_heartbeat_receiver::ClusterHeartbeatReceiver;
use crate::member::MemberStatus;

pub(crate) mod add_on_member_removed_listener;
pub(crate) mod add_on_member_up_listener;
pub(crate) mod get_cluster_core_ref_req;
mod leave_req;

trait ClusterMessage {}

pub(crate) mod cluster_user_action {

    use actor_core::actor::address::Address;
    use actor_core::actor::behavior::Behavior;
    use actor_core::actor::receive::Receive;
    use actor_core::actor::Actor;
    use actor_core::actor_ref::ActorRef;
    use actor_core::message::handler::MessageHandler;
    use actor_core::util::version::Version;
    use actor_core::Message;

    use crate::cluster_core_daemon::ClusterCoreDaemon;
    use crate::cluster_daemon::ClusterMessage;

    #[derive(Debug, Clone, Message, derive_more::Display)]
    #[display("JoinTo {{ address: {address} }}")]
    pub(crate) struct JoinTo {
        pub(crate) address: Address,
    }

    impl MessageHandler<ClusterCoreDaemon> for JoinTo {
        fn handle(
            actor: &mut ClusterCoreDaemon,
            ctx: &mut <ClusterCoreDaemon as Actor>::Context,
            message: Self,
            sender: Option<ActorRef>,
            _: &Receive<ClusterCoreDaemon>,
        ) -> anyhow::Result<Behavior<ClusterCoreDaemon>> {
            todo!()
        }
    }

    #[derive(Debug, Clone, Message, derive_more::Display)]
    #[cloneable]
    #[display("Leave {{ address: {address} }}")]
    pub(crate) struct Leave {
        pub(crate) address: Address,
    }

    impl MessageHandler<ClusterCoreDaemon> for Leave {
        fn handle(
            actor: &mut ClusterCoreDaemon,
            ctx: &mut <ClusterCoreDaemon as Actor>::Context,
            message: Self,
            sender: Option<ActorRef>,
            _: &Receive<ClusterCoreDaemon>,
        ) -> anyhow::Result<Behavior<ClusterCoreDaemon>> {
            todo!()
        }
    }

    impl ClusterMessage for Leave {}

    #[derive(Debug, Clone, Message, derive_more::Display)]
    #[cloneable]
    #[display("Down {{ address: {address} }}")]
    pub(crate) struct Down {
        pub(crate) address: Address,
    }

    impl ClusterMessage for Down {}

    impl MessageHandler<ClusterCoreDaemon> for Down {
        fn handle(
            actor: &mut ClusterCoreDaemon,
            ctx: &mut <ClusterCoreDaemon as Actor>::Context,
            message: Self,
            sender: Option<ActorRef>,
            _: &Receive<ClusterCoreDaemon>,
        ) -> anyhow::Result<Behavior<ClusterCoreDaemon>> {
            todo!()
        }
    }

    #[derive(Debug, Clone, Message, derive_more::Display)]
    #[cloneable]
    #[display("PrepareForShutdown")]
    pub(crate) struct PrepareForShutdown;

    impl ClusterMessage for PrepareForShutdown {}

    impl MessageHandler<ClusterCoreDaemon> for PrepareForShutdown {
        fn handle(
            actor: &mut ClusterCoreDaemon,
            ctx: &mut <ClusterCoreDaemon as Actor>::Context,
            message: Self,
            sender: Option<ActorRef>,
            _: &Receive<ClusterCoreDaemon>,
        ) -> anyhow::Result<Behavior<ClusterCoreDaemon>> {
            todo!()
        }
    }

    #[derive(Debug, Clone, Message, derive_more::Display)]
    #[cloneable]
    #[display("Shutdown")]
    pub(crate) struct SetAppVersionLater;

    impl MessageHandler<ClusterCoreDaemon> for SetAppVersionLater {
        fn handle(
            actor: &mut ClusterCoreDaemon,
            ctx: &mut <ClusterCoreDaemon as Actor>::Context,
            message: Self,
            sender: Option<ActorRef>,
            _: &Receive<ClusterCoreDaemon>,
        ) -> anyhow::Result<Behavior<ClusterCoreDaemon>> {
            todo!()
        }
    }

    #[derive(Debug, Clone, Message, derive_more::Display)]
    #[cloneable]
    #[display("SetAppVersion {{ app_version: {app_version} }}")]
    pub(crate) struct SetAppVersion {
        pub(crate) app_version: Version,
    }

    impl MessageHandler<ClusterCoreDaemon> for SetAppVersion {
        fn handle(
            actor: &mut ClusterCoreDaemon,
            ctx: &mut <ClusterCoreDaemon as Actor>::Context,
            message: Self,
            sender: Option<ActorRef>,
            _: &Receive<ClusterCoreDaemon>,
        ) -> anyhow::Result<Behavior<ClusterCoreDaemon>> {
            todo!()
        }
    }
}

#[derive(Debug)]
pub struct ClusterDaemon {
    core_supervisor: Option<ActorRef>,
    cluster_shutdown: Sender<()>,
}

impl Actor for ClusterDaemon {
    type Context = Context;

    fn started(&mut self, ctx: &mut Self::Context) -> anyhow::Result<()> {
        let cluster = Cluster::get(ctx.system()).clone();
        let myself = ctx.myself().clone();
        let cluster_shutdown = self.cluster_shutdown.clone();
        let coord_shutdown = CoordinatedShutdown::get(ctx.system());
        let phase_cluster_leave_timeout =
            CoordinatedShutdown::timeout(ctx.system(), PHASE_CLUSTER_LEAVE)
                .into_result()
                .context(format!("phase {} not found", PHASE_CLUSTER_LEAVE))?;
        coord_shutdown.add_task(ctx.system(), PHASE_CLUSTER_LEAVE, "leave", async move {
            if cluster.is_terminated() || cluster.self_member().status == MemberStatus::Removed {
                if let Some(_) = cluster_shutdown.send(()).await.err() {
                    debug!("send shutdown failed because receiver already closed");
                }
            } else {
                if let Some(error) = myself
                    .ask::<_, LeaveResp>(LeaveReq, phase_cluster_leave_timeout)
                    .await
                    .err()
                {
                    warn!("ask {} error {:?}", type_name::<LeaveReq>(), error);
                }
            }
        })?;
        Ok(())
    }

    fn stopped(&mut self, ctx: &mut Self::Context) -> anyhow::Result<()> {
        let _ = self.cluster_shutdown.send(()).await;
        let system = ctx.system().clone();
        let fut = {
            let coord_shutdown = CoordinatedShutdown::get(&system);
            coord_shutdown.run(ClusterDowningReason)
        };
        tokio::spawn(fut);
        Ok(())
    }

    fn receive(&self) -> actor_core::actor::receive::Receive<Self> {
        todo!()
    }
}

impl ClusterDaemon {
    pub(crate) fn new(context: &mut Context) -> anyhow::Result<Self> {
        let coord_shutdown = CoordinatedShutdown::get(context.system());
        let (cluster_shutdown_tx, mut cluster_shutdown_rx) = channel(1);
        coord_shutdown.add_task(
            context.system(),
            PHASE_CLUSTER_SHUTDOWN,
            "wait-shutdown",
            async move {
                let _ = cluster_shutdown_rx.recv().await;
            },
        )?;
        let daemon = Self {
            core_supervisor: None,
            cluster_shutdown: cluster_shutdown_tx,
        };
        Ok(daemon)
    }

    fn create_children(&mut self, context: &mut Context) -> anyhow::Result<()> {
        let core_supervisor = context.spawn(
            Props::new_with_ctx(|ctx| Ok(ClusterCoreSupervisor::new(ctx))),
            "core",
        )?;
        self.core_supervisor = Some(core_supervisor);
        context.spawn(
            ClusterHeartbeatReceiver::props(),
            ClusterHeartbeatReceiver::name(),
        )?;
        Ok(())
    }
}

pub(crate) mod internal_cluster_action {
    use actor_core::actor::behavior::Behavior;
    use actor_core::actor::receive::Receive;
    use actor_core::actor::Actor;
    use actor_core::message::handler::MessageHandler;
    use actor_core::message::DynMessage;
    use actor_core::Message;
    use ahash::HashSet;
    use imstr::ImString;

    use actor_core::actor::address::Address;
    use actor_core::actor::context::Context;
    use actor_core::actor_ref::ActorRef;
    use actor_core::util::version::Version;

    use crate::cluster_daemon::{ClusterDaemon, ClusterMessage};
    use crate::cluster_event::{ClusterDomainEvent, SubscriptionInitialStateMode};
    use crate::gossip::Gossip;
    use crate::unique_address::UniqueAddress;

    #[derive(Debug, Message, derive_more::Display)]
    #[display("Join {{ node: {node}, roles: {roles}, app_version: {app_version} }}")]
    pub(crate) struct Join {
        pub(crate) node: UniqueAddress,
        pub(crate) roles: HashSet<ImString>,
        pub(crate) app_version: Version,
    }

    impl ClusterMessage for Join {}

    impl MessageHandler<ClusterDaemon> for Join {
        fn handle(
            actor: &mut ClusterDaemon,
            ctx: &mut <ClusterDaemon as Actor>::Context,
            message: Self,
            sender: Option<ActorRef>,
            _: &Receive<ClusterDaemon>,
        ) -> anyhow::Result<Behavior<ClusterDaemon>> {
            todo!()
        }
    }

    #[derive(Debug, Message, derive_more::Display)]
    #[display("Welcome {{ from: {from}, gossip: {gossip} }}")]
    pub(crate) struct Welcome {
        pub(crate) from: UniqueAddress,
        pub(crate) gossip: Gossip,
    }

    impl ClusterMessage for Welcome {}

    impl MessageHandler<ClusterDaemon> for Welcome {
        fn handle(
            actor: &mut ClusterDaemon,
            ctx: &mut <ClusterDaemon as Actor>::Context,
            message: Self,
            sender: Option<ActorRef>,
            _: &Receive<ClusterDaemon>,
        ) -> anyhow::Result<Behavior<ClusterDaemon>> {
            todo!()
        }
    }

    #[derive(Debug, Message, derive_more::Display)]
    #[display("JoinSeedNodes {{ seed_nodes: {seed_nodes} }}")]
    pub(crate) struct JoinSeedNodes {
        pub(crate) seed_nodes: Vec<Address>,
    }

    impl MessageHandler<ClusterDaemon> for JoinSeedNodes {
        fn handle(
            actor: &mut ClusterDaemon,
            ctx: &mut <ClusterDaemon as Actor>::Context,
            message: Self,
            sender: Option<ActorRef>,
            _: &Receive<ClusterDaemon>,
        ) -> anyhow::Result<Behavior<ClusterDaemon>> {
            todo!()
        }
    }

    #[derive(Debug, Message, derive_more::Display)]
    #[display("JoinSeedNode")]
    pub(crate) struct JoinSeedNode;

    impl MessageHandler<ClusterDaemon> for JoinSeedNode {
        fn handle(
            actor: &mut ClusterDaemon,
            ctx: &mut <ClusterDaemon as Actor>::Context,
            message: Self,
            sender: Option<ActorRef>,
            _: &Receive<ClusterDaemon>,
        ) -> anyhow::Result<Behavior<ClusterDaemon>> {
            todo!()
        }
    }

    #[derive(Debug, Message, derive_more::Display)]
    #[display("Subscribe {{ subscriber: {subscriber}, initial_state_mode: {initial_state_mode}, to: {to} }}")]
    pub struct Subscribe {
        pub subscriber: ActorRef,
        pub initial_state_mode: SubscriptionInitialStateMode,
        pub to: &'static str,
        pub transform: Box<dyn Fn(Box<dyn ClusterDomainEvent>) -> DynMessage + Send>,
    }

    #[derive(Debug, Message, derive_more::Display)]
    #[display("Unsubscribe {{ subscriber: {subscriber}, to: {to} }}")]
    pub struct Unsubscribe {
        pub subscriber: ActorRef,
        pub to: Option<&'static str>,
    }
}

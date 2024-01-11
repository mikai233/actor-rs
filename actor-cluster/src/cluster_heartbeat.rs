use std::collections::HashSet;

use async_trait::async_trait;
use bincode::{Decode, Encode};
use tracing::trace;

use actor_core::{Actor, DynMessage, Message};
use actor_core::actor::actor_path::{ActorPath, TActorPath};
use actor_core::actor::actor_path::root_actor_path::RootActorPath;
use actor_core::actor::actor_ref::ActorRef;
use actor_core::actor::actor_ref_factory::ActorRefFactory;
use actor_core::actor::address::Address;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::event::EventBus;
use actor_derive::{EmptyCodec, MessageCodec};

use crate::cluster_daemon::ClusterDaemon;
use crate::cluster_event::ClusterEvent;
use crate::unique_address::UniqueAddress;

#[derive(Debug)]
pub(crate) struct ClusterHeartbeatSender {
    pub(crate) active_receivers: HashSet<UniqueAddress>,
    pub(crate) event_adapter: ActorRef,
}

#[async_trait]
impl Actor for ClusterHeartbeatSender {
    async fn pre_start(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        trace!("{} started", context.myself());
        context.system().event_stream().subscribe(self.event_adapter.clone(), std::any::type_name::<ClusterEvent>());
        Ok(())
    }

    async fn post_stop(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        trace!("{} stopped", context.myself());
        context.system().event_stream().unsubscribe(&self.event_adapter, std::any::type_name::<ClusterEvent>());
        Ok(())
    }
}

impl ClusterHeartbeatSender {
    pub(crate) fn new(context: &mut ActorContext) -> Self {
        let event_adapter = context.message_adapter::<ClusterEvent>(|m| {
            let m = DynMessage::user(HeartbeatSenderClusterEvent(m));
            Ok(m)
        });
        Self {
            active_receivers: Default::default(),
            event_adapter,
        }
    }

    pub(crate) fn name() -> &'static str {
        "heartbeat_sender"
    }
}

#[derive(Debug, EmptyCodec)]
struct HeartbeatSenderClusterEvent(ClusterEvent);

#[async_trait]
impl Message for HeartbeatSenderClusterEvent {
    type A = ClusterHeartbeatSender;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        todo!()
    }
}

#[derive(Debug, Default)]
pub(crate) struct ClusterHeartbeatReceiver;

#[async_trait]
impl Actor for ClusterHeartbeatReceiver {
    async fn pre_start(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        trace!("started {}", context.myself());
        Ok(())
    }
}

impl ClusterHeartbeatReceiver {
    pub(crate) fn name() -> &'static str {
        "heartbeat_receiver"
    }

    pub(crate) fn path(address: Address) -> ActorPath {
        RootActorPath::new(address, "/").descendant(vec!["system", "cluster", Self::name()]).into()
    }
}

#[derive(Debug, Encode, Decode, MessageCodec)]
struct Heartbeat {
    pub(crate) from: Address,
}

#[async_trait]
impl Message for Heartbeat {
    type A = ClusterDaemon;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        todo!()
    }
}

#[derive(Debug, Encode, Decode, MessageCodec)]
struct HeartbeatRsp {
    pub(crate) from: UniqueAddress,
}

#[async_trait]
impl Message for HeartbeatRsp {
    type A = ClusterDaemon;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        todo!()
    }
}

#[derive(Debug, EmptyCodec)]
struct HeartbeatTick;

#[async_trait]
impl Message for HeartbeatTick {
    type A = ClusterDaemon;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        todo!()
    }
}
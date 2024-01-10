use std::collections::HashSet;

use async_trait::async_trait;
use bincode::{Decode, Encode};
use tracing::trace;

use actor_core::{Actor, Message};
use actor_core::actor::actor_path::{ActorPath, TActorPath};
use actor_core::actor::actor_path::root_actor_path::RootActorPath;
use actor_core::actor::address::Address;
use actor_core::actor::context::{ActorContext, Context};
use actor_derive::{EmptyCodec, MessageCodec};

use crate::cluster_daemon::ClusterDaemon;
use crate::unique_address::UniqueAddress;

#[derive(Debug, Default)]
pub(crate) struct ClusterHeartbeatSender {
    pub(crate) active_receivers: HashSet<UniqueAddress>,
}

#[async_trait]
impl Actor for ClusterHeartbeatSender {
    async fn pre_start(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        trace!("{} started", context.myself());
        Ok(())
    }
}

impl ClusterHeartbeatSender {
    pub(crate) fn name() -> &'static str {
        "heartbeat_sender"
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
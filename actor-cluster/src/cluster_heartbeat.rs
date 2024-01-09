use async_trait::async_trait;
use bincode::{Decode, Encode};

use actor_core::{Actor, Message};
use actor_core::actor::address::Address;
use actor_core::actor::context::ActorContext;
use actor_derive::{EmptyCodec, MessageCodec};

use crate::cluster_daemon::ClusterDaemon;
use crate::unique_address::UniqueAddress;

#[derive(Debug)]
pub(crate) struct ClusterHeartbeatSender {}

#[async_trait]
impl Actor for ClusterHeartbeatSender {}

impl ClusterHeartbeatSender {}

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
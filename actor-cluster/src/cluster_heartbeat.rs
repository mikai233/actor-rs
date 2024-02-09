use std::collections::HashSet;
use std::time::Duration;

use async_trait::async_trait;
use bincode::{Decode, Encode};
use tracing::trace;

use actor_core::{Actor, DynMessage, Message};
use actor_core::actor::actor_path::{ActorPath, TActorPath};
use actor_core::actor::actor_path::root_actor_path::RootActorPath;
use actor_core::actor::actor_ref::{ActorRef, ActorRefExt};
use actor_core::actor::actor_ref_factory::ActorRefFactory;
use actor_core::actor::actor_selection::ActorSelectionPath;
use actor_core::actor::address::Address;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::scheduler::ScheduleKey;
use actor_derive::{CEmptyCodec, CMessageCodec, EmptyCodec, MessageCodec};

use crate::cluster::Cluster;
use crate::cluster_event::ClusterEvent;
use crate::member::{Member, MemberStatus};
use crate::unique_address::UniqueAddress;

#[derive(Debug)]
pub(crate) struct ClusterHeartbeatSender {
    self_member: Option<Member>,
    active_receivers: HashSet<UniqueAddress>,
    event_adapter: ActorRef,
    key: Option<ScheduleKey>,
}

#[async_trait]
impl Actor for ClusterHeartbeatSender {
    async fn started(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        trace!("{} started", context.myself());
        Cluster::get(context.system()).subscribe_cluster_event(self.event_adapter.clone());
        let myself = context.myself().clone();
        let key = context.system().scheduler().schedule_with_fixed_delay(None, Duration::from_secs(5), move || {
            myself.cast_ns(HeartbeatTick);
        });
        self.key = Some(key);
        Ok(())
    }

    async fn stopped(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        trace!("{} stopped", context.myself());
        if let Some(key) = self.key.take() {
            key.cancel();
        }
        Cluster::get(context.system()).unsubscribe_cluster_event(&self.event_adapter);
        Ok(())
    }
}

impl ClusterHeartbeatSender {
    pub(crate) fn new(context: &mut ActorContext) -> Self {
        let event_adapter = context.message_adapter(|m| DynMessage::user(HeartbeatSenderClusterEvent(m)));
        Self {
            active_receivers: Default::default(),
            event_adapter,
            self_member: None,
            key: None,
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
        trace!("{} {:?}", context.myself(), self);
        match self.0 {
            ClusterEvent::MemberUp(m) => {
                if actor.self_member.as_ref().is_some_and(|sm| sm.addr == m.addr) {
                    actor.self_member = Some(m.clone());
                }
                actor.active_receivers.insert(m.addr);
            }
            ClusterEvent::MemberPrepareForLeaving(_) => {}
            ClusterEvent::MemberLeaving(_) => {}
            ClusterEvent::MemberDowned(m) => {
                if actor.self_member.as_ref().is_some_and(|sm| sm.addr == m.addr) {
                    actor.self_member = Some(m.clone());
                }
                actor.active_receivers.remove(&m.addr);
            }
            ClusterEvent::CurrentClusterState { members, self_member } => {
                actor.self_member = Some(self_member);
                actor.active_receivers.extend(members.into_keys());
            }
            ClusterEvent::EtcdUnreachable => {}
            ClusterEvent::MemberRemoved(_) => {}
        }
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct ClusterHeartbeatReceiver {
    self_member: Option<Member>,
    event_adapter: ActorRef,
}

#[async_trait]
impl Actor for ClusterHeartbeatReceiver {
    async fn started(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        trace!("started {}", context.myself());
        Cluster::get(context.system()).subscribe_cluster_event(self.event_adapter.clone());
        Ok(())
    }
}

impl ClusterHeartbeatReceiver {
    pub(crate) fn new(context: &mut ActorContext) -> Self {
        let event_adapter = context.message_adapter(|m| DynMessage::user(HeartbeatReceiverClusterEvent(m)));
        Self {
            self_member: None,
            event_adapter,
        }
    }

    pub(crate) fn name() -> &'static str {
        "heartbeat_receiver"
    }

    pub(crate) fn path(address: Address) -> ActorPath {
        RootActorPath::new(address, "/").descendant(vec!["system", "cluster", Self::name()]).into()
    }
}

#[derive(Debug, EmptyCodec)]
struct HeartbeatReceiverClusterEvent(ClusterEvent);

#[async_trait]
impl Message for HeartbeatReceiverClusterEvent {
    type A = ClusterHeartbeatReceiver;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        trace!("{} {:?}", context.myself(), self);
        match self.0 {
            ClusterEvent::MemberUp(m) => {
                if actor.self_member.as_ref().is_some_and(|sm| sm.addr == m.addr) {
                    actor.self_member = Some(m.clone());
                }
            }
            ClusterEvent::MemberPrepareForLeaving(_) => {}
            ClusterEvent::MemberLeaving(_) => {}
            ClusterEvent::MemberDowned(m) => {
                if actor.self_member.as_ref().is_some_and(|sm| sm.addr == m.addr) {
                    actor.self_member = Some(m.clone());
                }
            }
            ClusterEvent::CurrentClusterState { self_member, .. } => {
                actor.self_member = Some(self_member);
            }
            ClusterEvent::EtcdUnreachable => {}
            ClusterEvent::MemberRemoved(_) => {}
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Encode, Decode, CMessageCodec)]
pub(crate) struct Heartbeat {
    from: UniqueAddress,
}

#[async_trait]
impl Message for Heartbeat {
    type A = ClusterHeartbeatReceiver;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        trace!("{} recv Heartbeat from {}", context.myself(), self.from);
        if let Some(self_member) = &actor.self_member {
            if self_member.status == MemberStatus::Up {
                context.sender().unwrap().tell(DynMessage::user(HeartbeatRsp { from: self_member.addr.clone() }), ActorRef::no_sender());
            }
        }
        Ok(())
    }
}

#[derive(Debug, Encode, Decode, MessageCodec)]
pub(crate) struct HeartbeatRsp {
    from: UniqueAddress,
}

#[async_trait]
impl Message for HeartbeatRsp {
    type A = ClusterHeartbeatSender;

    async fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
        trace!("{} recv HeartbeatRsp from {}", context.myself(), self.from);
        Ok(())
    }
}

#[derive(Debug, Clone, CEmptyCodec)]
struct HeartbeatTick;

#[async_trait]
impl Message for HeartbeatTick {
    type A = ClusterHeartbeatSender;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        if let Some(self_member) = &actor.self_member {
            if self_member.status == MemberStatus::Up {
                for receiver in &actor.active_receivers {
                    let sel = context.actor_selection(ActorSelectionPath::FullPath(ClusterHeartbeatReceiver::path(receiver.address.clone())))?;
                    sel.tell(DynMessage::user(Heartbeat { from: self_member.addr.clone() }), Some(context.myself().clone()));
                }
            }
        }
        Ok(())
    }
}
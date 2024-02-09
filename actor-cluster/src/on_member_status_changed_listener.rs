use async_trait::async_trait;

use actor_core::{Actor, DynMessage, Message};
use actor_core::actor::actor_ref::ActorRef;
use actor_core::actor::actor_ref_factory::ActorRefFactory;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::ext::option_ext::OptionExt;
use actor_derive::EmptyCodec;

use crate::cluster::Cluster;
use crate::cluster_event::ClusterEvent;
use crate::member::MemberStatus;

pub(crate) struct OnMemberStatusChangedListener {
    event_adapter: ActorRef,
    callback: Option<Box<dyn FnOnce() + Send>>,
    status: MemberStatus,
}

impl OnMemberStatusChangedListener {
    pub(crate) fn new(context: &mut ActorContext, status: MemberStatus) -> Self {
        let event_adapter = context.message_adapter(|m| DynMessage::user(ClusterEventWrap(m)));
        Self {
            event_adapter,
            callback: None,
            status,
        }
    }
}

#[async_trait]
impl Actor for OnMemberStatusChangedListener {
    async fn started(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        let cluster = Cluster::get(context.system());
        cluster.subscribe_cluster_event(self.event_adapter.clone());
        debug_assert!(matches!(self.status,MemberStatus::Up) || matches!(self.status,MemberStatus::Removed));
        Ok(())
    }

    async fn stopped(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        if matches!(self.status,MemberStatus::Removed) {
            self.callback.take().into_foreach(|cb| cb());
        };
        let cluster = Cluster::get(context.system());
        cluster.unsubscribe_cluster_event(&self.event_adapter);
        Ok(())
    }
}

#[derive(Debug, EmptyCodec)]
struct ClusterEventWrap(ClusterEvent);

#[async_trait]
impl Message for ClusterEventWrap {
    type A = OnMemberStatusChangedListener;

    async fn handle(self: Box<Self>, _context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        match self.0 {
            ClusterEvent::MemberUp(_) if matches!(actor.status, MemberStatus::Up) => {
                actor.callback.take().into_foreach(|callback| callback());
            }
            ClusterEvent::MemberRemoved(_) if matches!(actor.status,MemberStatus::Removed) => {
                actor.callback.take().into_foreach(|callback| callback());
            }
            _ => panic!("unreachable")
        }
        Ok(())
    }
}

#[derive(EmptyCodec)]
pub(crate) struct AddStatusCallback(pub(crate) Box<dyn FnOnce() + Send>);

#[async_trait]
impl Message for AddStatusCallback {
    type A = OnMemberStatusChangedListener;

    async fn handle(self: Box<Self>, _context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        actor.callback = Some(self.0);
        Ok(())
    }
}
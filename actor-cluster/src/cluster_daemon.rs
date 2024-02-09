use std::collections::{HashMap, HashSet};
use std::collections::hash_map::Entry;
use std::fmt::Debug;
use std::sync::RwLockWriteGuard;
use std::time::Duration;

use async_trait::async_trait;
use etcd_client::{EventType, LeaseTimeToLiveOptions, PutOptions, WatchOptions, WatchResponse};
use tracing::{debug, error, info, trace};

use actor_core::{Actor, DynMessage, Message};
use actor_core::actor::actor_ref::{ActorRef, ActorRefExt};
use actor_core::actor::actor_ref_factory::ActorRefFactory;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::props::Props;
use actor_core::event::EventBus;
use actor_core::ext::etcd_client::EtcdClient;
use actor_core::ext::option_ext::OptionExt;
use actor_derive::EmptyCodec;

use crate::cluster::Cluster;
use crate::cluster_event::ClusterEvent;
use crate::cluster_heartbeat::{ClusterHeartbeatReceiver, ClusterHeartbeatSender};
use crate::etcd_watcher::{EtcdWatcher, WatchResp};
use crate::lease_keeper::{LeaseKeepAliveFailed, LeaseKeeper};
use crate::member::{Member, MemberStatus};
use crate::on_member_status_changed_listener::{AddStatusCallback, OnMemberStatusChangedListener};
use crate::unique_address::UniqueAddress;

#[derive(Debug)]
pub struct ClusterDaemon {
    pub(crate) client: EtcdClient,
    pub(crate) self_addr: UniqueAddress,
    pub(crate) roles: HashSet<String>,
    pub(crate) transport: ActorRef,
    pub(crate) key_addr: HashMap<String, UniqueAddress>,
    pub(crate) cluster: Option<Cluster>,
}

#[async_trait]
impl Actor for ClusterDaemon {
    async fn started(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        trace!("{} started", context.myself());
        let cluster = Cluster::get(context.system()).clone();
        self.cluster = Some(cluster);
        context.spawn(Props::create(|context|
            ClusterHeartbeatSender::new(context)), ClusterHeartbeatSender::name(),
        )?;
        context.spawn(Props::create(|context|
            ClusterHeartbeatReceiver::new(context)), ClusterHeartbeatReceiver::name(),
        )?;
        let adapter = context.message_adapter(|m| DynMessage::user(WatchRespWrap(m)));
        self.spawn_member_watcher(context, adapter)?;
        let lease_id = self.spawn_lease_keeper(context).await?;
        let member = Member::new(
            self.self_addr.clone(),
            MemberStatus::Up,
            self.roles.clone(),
            lease_id,
        );
        self.update_member_to_etcd(&member).await?;
        Ok(())
    }

    async fn stopped(&mut self, _context: &mut ActorContext) -> anyhow::Result<()> {
        let _ = self.self_down().await;
        Ok(())
    }
}

impl ClusterDaemon {
    fn lease_path(&self) -> String {
        format!("actor/{}/cluster/lease", self.self_addr.system_name())
    }

    fn spawn_watcher(context: &mut ActorContext, name: impl Into<String>, adapter: ActorRef, key: String, options: Option<WatchOptions>, client: EtcdClient) -> anyhow::Result<()> {
        context.spawn(Props::create(move |ctx| {
            EtcdWatcher::new(
                ctx.myself().clone(),
                client.clone(),
                key.clone(),
                options.clone(),
                adapter.clone(),
            )
        }), name.into())?;
        Ok(())
    }

    async fn spawn_lease_keeper(&mut self, context: &mut ActorContext) -> anyhow::Result<i64> {
        let resp = self.client.lease_grant(60, None).await?;
        let lease_id = resp.id();
        let client = self.client.clone();
        let receiver = context.message_adapter::<LeaseKeepAliveFailed>(|m| DynMessage::user(LeaseFailed));
        context.spawn(
            Props::create(move |_| {
                LeaseKeeper::new(client.clone(), resp.id(), receiver.clone(), Duration::from_secs(3))
            }),
            "lease_keeper",
        )?;
        Ok(lease_id)
    }

    async fn update_member_to_etcd(&mut self, member: &Member) -> anyhow::Result<()> {
        let socket_addr = member.addr.socket_addr_with_uid();
        let member_addr = socket_addr.as_result()?;
        let lease_path = self.lease_path();
        let key = format!("{}/{}", lease_path, member_addr);
        let value = serde_json::to_vec(&member)?;
        let put_options = PutOptions::new().with_lease(member.lease);
        self.client.put(key, value, Some(put_options)).await?;
        Ok(())
    }

    async fn self_removed(&mut self) -> anyhow::Result<()> {
        let self_member = {
            let cluster = self.cluster.as_result_mut()?;
            *cluster.members_write() = HashMap::new();
            let mut self_member = cluster.self_member_write();
            self_member.status = MemberStatus::Removed;
            self_member.clone()
        };
        info!("{:?} self removed", self_member);
        Ok(())
    }

    async fn self_down(&mut self) -> anyhow::Result<()> {
        let self_member = {
            let cluster = self.cluster.as_result_mut()?;
            let mut self_member = cluster.self_member_write();
            self_member.status = MemberStatus::Down;
            self_member.clone()
        };
        info!("{:?} self down", self_member);
        Ok(())
    }

    fn spawn_member_watcher(&mut self, context: &mut ActorContext, adapter: ActorRef) -> anyhow::Result<()> {
        Self::spawn_watcher(
            context,
            "member_watcher",
            adapter.clone(),
            self.lease_path(),
            Some(WatchOptions::new().with_prefix()),
            self.client.clone(),
        )?;
        Ok(())
    }

    async fn respawn_lease_keeper(&mut self, context: &mut ActorContext) {
        match self.spawn_lease_keeper(context).await {
            Ok(lease_id) => {
                let member = Member::new(
                    self.self_addr.clone(),
                    MemberStatus::Up,
                    self.roles.clone(),
                    lease_id,
                );
                if let Some(error) = self.update_member_to_etcd(&member).await.err() {
                    error!("{} update self member error {:?}", context.myself(), error);
                    context.myself().cast_ns(LeaseFailed);
                }
            }
            Err(error) => {
                error!("{} spawn lease keeper error {:?}", context.myself(), error);
                context.myself().cast_ns(LeaseFailed);
            }
        }
    }
}

#[derive(Debug, EmptyCodec)]
struct WatchRespWrap(WatchResp);

#[async_trait]
impl Message for WatchRespWrap {
    type A = ClusterDaemon;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let WatchResp { key, resp } = self.0;
        if key == actor.lease_path() {
            Self::update_member_status(context, actor, resp).await?;
        }
        Ok(())
    }
}

impl WatchRespWrap {
    /// update local member status form etcd
    async fn update_member_status(context: &mut ActorContext, actor: &mut ClusterDaemon, resp: WatchResponse) -> anyhow::Result<()> {
        for event in resp.events() {
            if let Some(kv) = event.kv() {
                let cluster = Cluster::get(context.system());
                let stream = context.system().event_stream();
                match event.event_type() {
                    EventType::Put => {
                        let member = serde_json::from_slice::<Member>(kv.value())?;
                        actor.key_addr.insert(kv.key_str()?.to_string(), member.addr.clone());
                        debug!("{} update member {:?}", context.myself(), member);
                        if member.addr == actor.self_addr {
                            *cluster.self_member_write() = member.clone();
                        }
                        match member.status {
                            MemberStatus::Up => {
                                if Self::update_member(member.clone(), cluster.members_write()) {
                                    stream.publish(DynMessage::orphan(ClusterEvent::member_up(member)))?;
                                }
                            }
                            MemberStatus::PrepareForLeaving => {
                                if Self::update_member(member.clone(), cluster.members_write()) {
                                    stream.publish(DynMessage::orphan(ClusterEvent::member_prepare_for_leaving(member)))?;
                                }
                            }
                            MemberStatus::Leaving => {
                                if Self::update_member(member.clone(), cluster.members_write()) {
                                    stream.publish(DynMessage::orphan(ClusterEvent::member_leaving(member)))?;
                                }
                            }
                            MemberStatus::Removed => {
                                if cluster.members_write().remove(&member.addr).is_some() {
                                    if member.addr == actor.self_addr {
                                        context.myself().cast_ns(SelfRemoved);
                                    }
                                    stream.publish(DynMessage::orphan(ClusterEvent::member_removed(member)))?;
                                }
                            }
                            MemberStatus::Down => {
                                stream.publish(DynMessage::orphan(ClusterEvent::member_downed(member)))?;
                            }
                        }
                    }
                    EventType::Delete => {
                        if let Some(addr) = actor.key_addr.remove(kv.key_str()?) {
                            if let Some(mut member) = cluster.members_write().remove(&addr) {
                                member.status = MemberStatus::Removed;
                                if addr == actor.self_addr {
                                    context.myself().cast_ns(SelfRemoved);
                                }
                                stream.publish(DynMessage::orphan(ClusterEvent::member_removed(member.clone())))?;
                                member.status = MemberStatus::Down;
                                if addr == actor.self_addr {
                                    context.myself().cast_ns(SelfDown);
                                }
                                stream.publish(DynMessage::orphan(ClusterEvent::member_downed(member)))?;
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn update_member(member: Member, mut members: RwLockWriteGuard<HashMap<UniqueAddress, Member>>) -> bool {
        match members.entry(member.addr.clone()) {
            Entry::Occupied(mut o) => {
                if &member != o.get() {
                    o.insert(member);
                    true
                } else {
                    false
                }
            }
            Entry::Vacant(v) => {
                v.insert(member);
                true
            }
        }
    }
}

#[derive(Debug, EmptyCodec)]
struct LeaseFailed;

#[async_trait]
impl Message for LeaseFailed {
    type A = ClusterDaemon;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        trace!("{} lease failed", context.myself());
        actor.respawn_lease_keeper(context).await;
        Ok(())
    }
}

#[derive(Debug, EmptyCodec)]
struct SelfRemoved;

#[async_trait]
impl Message for SelfRemoved {
    type A = ClusterDaemon;

    async fn handle(self: Box<Self>, _context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        actor.self_removed().await?;
        Ok(())
    }
}

#[derive(Debug, EmptyCodec)]
struct SelfDown;

#[async_trait]
impl Message for SelfDown {
    type A = ClusterDaemon;

    async fn handle(self: Box<Self>, _context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        actor.self_down().await?;
        Ok(())
    }
}

#[derive(EmptyCodec)]
pub(crate) struct AddOnMemberUpListener(Box<dyn FnOnce() + Send>);

#[async_trait]
impl Message for AddOnMemberUpListener {
    type A = ClusterDaemon;

    async fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
        let listener = context.spawn_anonymous(Props::create(|context| {
            OnMemberStatusChangedListener::new(context, MemberStatus::Up)
        }))?;
        listener.cast_ns(AddStatusCallback(self.0));
        trace!("{} add callback on member up", context.myself());
        Ok(())
    }
}

#[derive(EmptyCodec)]
pub(crate) struct AddOnMemberRemovedListener(pub(crate) Box<dyn FnOnce() + Send>);

#[async_trait]
impl Message for AddOnMemberRemovedListener {
    type A = ClusterDaemon;

    async fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
        let listener = context.spawn_anonymous(Props::create(|context| {
            OnMemberStatusChangedListener::new(context, MemberStatus::Removed)
        }))?;
        listener.cast_ns(AddStatusCallback(self.0));
        trace!("{} add callback on member removed", context.myself());
        Ok(())
    }
}
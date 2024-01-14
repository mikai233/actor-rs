use std::collections::{HashMap, HashSet};
use std::collections::hash_map::Entry;
use std::fmt::{Debug, Formatter};
use std::sync::RwLockWriteGuard;
use std::time::Duration;

use async_trait::async_trait;
use etcd_client::{Client, EventType, PutOptions, WatchOptions, WatchResponse};
use tracing::{debug, error, info, trace};

use actor_core::{Actor, DynMessage, Message};
use actor_core::actor::actor_ref::{ActorRef, ActorRefExt};
use actor_core::actor::actor_ref_factory::ActorRefFactory;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::props::Props;
use actor_core::event::EventBus;
use actor_derive::EmptyCodec;

use crate::cluster::Cluster;
use crate::cluster_event::ClusterEvent;
use crate::cluster_heartbeat::{ClusterHeartbeatReceiver, ClusterHeartbeatSender};
use crate::ewatcher::{KeyWatcher, WatchResp};
use crate::lease_keeper::{LeaseKeepAliveFailed, LeaseKeeper};
use crate::member::{Member, MemberStatus};
use crate::unique_address::UniqueAddress;

pub struct ClusterDaemon {
    pub(crate) eclient: Client,
    pub(crate) self_addr: UniqueAddress,
    pub(crate) roles: HashSet<String>,
    pub(crate) transport: ActorRef,
    pub(crate) lease_id: i64,
    pub(crate) key_addr: HashMap<String, UniqueAddress>,
    pub(crate) cluster: Option<Cluster>,
}

impl Debug for ClusterDaemon {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_struct("ClusterDaemon")
            .field("eclient", &"..")
            .field("self_addr", &self.self_addr)
            .field("roles", &self.roles)
            .field("transport", &self.transport)
            .field("lease_id", &self.lease_id)
            .field("key_addr", &self.key_addr)
            .finish()
    }
}

#[async_trait]
impl Actor for ClusterDaemon {
    async fn pre_start(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        trace!("{} started", context.myself());
        let cluster = Cluster::get(context.system()).clone();
        self.cluster = Some(cluster);
        context.spawn_actor(Props::create(|context| ClusterHeartbeatSender::new(context)), ClusterHeartbeatSender::name())?;
        context.spawn_actor(Props::create(|context| ClusterHeartbeatReceiver::new(context)), ClusterHeartbeatReceiver::name())?;
        let adapter = context.message_adapter::<WatchResp>(|m| {
            let m = DynMessage::user(WatchRespWrap(m));
            Ok(m)
        });
        self.spawn_lease_watcher(context, adapter)?;
        let lease_id = self.spawn_lease_keeper(context).await?;
        self.lease_id = lease_id;
        let member = Member::new(self.self_addr.clone(), MemberStatus::Up, self.roles.clone());
        self.update_self_member(&member).await?;
        Ok(())
    }

    async fn post_stop(&mut self, _context: &mut ActorContext) -> anyhow::Result<()> {
        let _ = self.self_removed().await;
        Ok(())
    }
}

impl ClusterDaemon {
    fn lease_path(&self) -> String {
        format!("actor/{}/cluster/lease", self.self_addr.system_name())
    }

    fn spawn_watcher(context: &mut ActorContext, name: impl Into<String>, adapter: ActorRef, key: String, options: Option<WatchOptions>, eclient: Client) -> anyhow::Result<()> {
        context.spawn_actor(Props::create(move |ctx| {
            KeyWatcher::new(
                ctx.myself().clone(),
                eclient.clone(),
                key.clone(),
                options.clone(),
                adapter.clone(),
            )
        }), name.into())?;
        Ok(())
    }

    async fn spawn_lease_keeper(&mut self, context: &mut ActorContext) -> anyhow::Result<i64> {
        let resp = self.eclient.lease_grant(60, None).await?;
        let lease_id = resp.id();
        let eclient = self.eclient.clone();
        let receiver = context.message_adapter::<LeaseKeepAliveFailed>(|m| {
            let m = DynMessage::user(LeaseFailed(m));
            Ok(m)
        });
        context.spawn_actor(
            Props::create(move |_| { LeaseKeeper::new(eclient.clone(), resp.id(), receiver.clone(), Duration::from_secs(3)) }),
            "lease_keeper",
        )?;
        Ok(lease_id)
    }

    async fn update_self_member(&mut self, member: &Member) -> anyhow::Result<()> {
        let key = format!("{}/{}", self.lease_path(), self.self_addr.socket_addr_with_uid().unwrap());
        let value = serde_json::to_vec(&member)?;
        let put_options = PutOptions::new().with_lease(self.lease_id);
        self.eclient.put(key, value, Some(put_options)).await?;
        Ok(())
    }

    async fn self_removed(&mut self) -> anyhow::Result<()> {
        let self_member = {
            let cluster = self.cluster.as_mut().unwrap();
            *cluster.members_write() = HashMap::new();
            let mut self_member = cluster.self_member_write();
            self_member.status = MemberStatus::Down;
            self_member.clone()
        };
        self.update_self_member(&self_member).await?;
        info!("{:?} self removed", self_member);
        Ok(())
    }

    fn spawn_lease_watcher(&mut self, context: &mut ActorContext, adapter: ActorRef) -> anyhow::Result<()> {
        Self::spawn_watcher(
            context,
            "lease_watcher",
            adapter.clone(),
            self.lease_path(),
            Some(WatchOptions::new().with_prefix()),
            self.eclient.clone(),
        )?;
        Ok(())
    }

    async fn respawn_lease_keeper(&mut self, context: &mut ActorContext) {
        match self.spawn_lease_keeper(context).await {
            Ok(lease_id) => {
                self.lease_id = lease_id;
                let member = Member::new(self.self_addr.clone(), MemberStatus::Up, self.roles.clone());
                if let Some(error) = self.update_self_member(&member).await.err() {
                    error!("{} update self member error {:?}", context.myself(), error);
                    context.myself().cast_ns(ReleaseFailed);
                }
            }
            Err(error) => {
                error!("{} spawn lease keeper error {:?}", context.myself(), error);
                context.myself().cast_ns(ReleaseFailed);
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
    async fn update_member_status(context: &mut ActorContext, actor: &mut ClusterDaemon, resp: WatchResponse) -> anyhow::Result<()> {
        for event in resp.events() {
            if let Some(kv) = event.kv() {
                let cluster = Cluster::get(context.system());
                match event.event_type() {
                    EventType::Put => {
                        let member = serde_json::from_slice::<Member>(kv.value())?;
                        actor.key_addr.insert(kv.key_str()?.to_string(), member.addr.clone());
                        debug!("{} update member {:?}", context.myself(), member);
                        if member.addr == actor.self_addr {
                            *cluster.self_member_write() = member.clone();
                        }
                        let stream = context.system().event_stream();
                        match member.status {
                            MemberStatus::Up => {
                                let updated = Self::update_member(member.clone(), cluster.members_write())?;
                                if updated {
                                    stream.publish(DynMessage::orphan(ClusterEvent::member_up(member)))?;
                                }
                            }
                            MemberStatus::Down => {
                                if cluster.members_write().remove(&member.addr).is_some() {
                                    if member.addr == actor.self_addr {
                                        context.myself().cast_ns(SelfRemoved);
                                    }
                                    stream.publish(DynMessage::orphan(ClusterEvent::member_downed(member)))?;
                                }
                            }
                        }
                    }
                    EventType::Delete => {
                        if let Some(addr) = actor.key_addr.remove(kv.key_str()?) {
                            if let Some(mut member) = cluster.members_write().remove(&addr) {
                                member.status = MemberStatus::Down;
                                info!("{} delete member {:?}", context.myself(), member);
                                if addr == actor.self_addr {
                                    context.myself().cast_ns(SelfRemoved);
                                }
                                let event = ClusterEvent::member_downed(member);
                                context.system().event_stream().publish(DynMessage::orphan(event))?;
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn update_member(member: Member, mut members: RwLockWriteGuard<HashMap<UniqueAddress, Member>>) -> anyhow::Result<bool> {
        let updated = match members.entry(member.addr.clone()) {
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
        };
        Ok(updated)
    }
}

#[derive(Debug, EmptyCodec)]
struct LeaseFailed(LeaseKeepAliveFailed);

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
struct ReleaseFailed;

#[async_trait]
impl Message for ReleaseFailed {
    type A = ClusterDaemon;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        trace!("{} release failed", context.myself());
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
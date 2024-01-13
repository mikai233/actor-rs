use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt::{Debug, Formatter};
use std::net::SocketAddrV4;
use std::ops::Sub;
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
use actor_core::ext::option_ext::OptionExt;
use actor_derive::EmptyCodec;

use crate::cluster::Cluster;
use crate::cluster_event::{ClusterEvent, CurrentClusterState, MemberEvent};
use crate::cluster_heartbeat::{ClusterHeartbeatReceiver, ClusterHeartbeatSender};
use crate::cluster_node::ClusterNode;
use crate::ewatcher::{EWatcher, EWatchResp};
use crate::lease_keeper::{LeaseKeepAliveFailed, LeaseKeeper};
use crate::member::{Member, MemberStatus};
use crate::unique_address::UniqueAddress;

pub struct ClusterDaemon {
    pub(crate) eclient: Client,
    pub(crate) self_addr: UniqueAddress,
    pub(crate) roles: HashSet<String>,
    pub(crate) transport: ActorRef,
    pub(crate) lease_id: Option<i64>,
    pub(crate) key_addr: HashMap<String, UniqueAddress>,
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
        let heartbeat_sender = context.spawn_actor(Props::create(|context| ClusterHeartbeatSender::new(context)), ClusterHeartbeatSender::name())?;
        let heartbeat_receiver = context.spawn_actor(Props::create(|_| ClusterHeartbeatReceiver::default()), ClusterHeartbeatReceiver::name())?;
        let cluster_node = self.get_cluster_node().await?;
        {
            let cluster = Cluster::get(context.system());
            *cluster.node().write().unwrap() = cluster_node;
        }
        let adapter = context.message_adapter::<EWatchResp>(|m| {
            let m = DynMessage::user(WatchResp(m));
            Ok(m)
        });
        self.spawn_node_watcher(context, &adapter)?;
        self.spawn_lease_watcher(context, adapter)?;
        if self.is_addr_in_cluster(context, self.self_addr.address.addr.as_ref().unwrap()) {
            let lease_id = self.spawn_lease_keeper(context).await?;
            self.lease_id = Some(lease_id);
            self.put_self_member().await?;
        }
        Ok(())
    }

    async fn post_stop(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        let cluster = Cluster::get(context.system());
        self.self_removed(cluster.state().write().unwrap(), cluster.self_member().write().unwrap());
        Ok(())
    }
}

impl ClusterDaemon {
    fn node_path(&self) -> String {
        format!("actor/{}/cluster/node", self.self_addr.system_name())
    }

    fn lease_path(&self) -> String {
        format!("actor/{}/cluster/lease", self.self_addr.system_name())
    }

    fn spawn_watcher(context: &mut ActorContext, name: impl Into<String>, adapter: ActorRef, key: String, options: Option<WatchOptions>, eclient: Client) -> anyhow::Result<()> {
        context.spawn_actor(Props::create(move |ctx| {
            EWatcher::new(
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

    //TODO shutdown actor system when first get etcd config error, do not retry
    async fn get_cluster_node(&mut self) -> anyhow::Result<ClusterNode> {
        let node_path = self.node_path();
        let resp = self.eclient.get(node_path.as_bytes(), None).await?;
        let kv = resp.kvs().first();
        let kv = kv.as_result()?;
        let cluster_node = serde_json::from_slice::<ClusterNode>(kv.value())?;
        Ok(cluster_node)
    }

    fn is_addr_in_cluster(&self, context: &mut ActorContext, addr: &SocketAddrV4) -> bool {
        let cluster = Cluster::get(context.system());
        let cluster_node = cluster.node().read().unwrap();
        cluster_node.addrs.contains(addr)
    }

    async fn put_self_member(&mut self) -> anyhow::Result<()> {
        let key_to_lease = format!("{}/{}", self.lease_path(), self.self_addr.socket_addr_with_uid().unwrap());
        let member = Member::new(self.self_addr.clone(), MemberStatus::Up, self.roles.clone());
        let member_json = serde_json::to_vec(&member)?;
        let put_options = PutOptions::new().with_lease(self.lease_id.unwrap());
        self.eclient.put(key_to_lease, member_json, Some(put_options)).await?;
        Ok(())
    }

    fn self_removed(&mut self, mut state: RwLockWriteGuard<CurrentClusterState>, mut self_member: RwLockWriteGuard<Member>) {
        *state = CurrentClusterState::default();
        self_member.status = MemberStatus::Removed;
        info!("{:?} self removed", self_member);
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

    fn spawn_node_watcher(&mut self, context: &mut ActorContext, adapter: &ActorRef) -> anyhow::Result<()> {
        Self::spawn_watcher(context, "node_watcher", adapter.clone(), self.node_path(), None, self.eclient.clone())?;
        Ok(())
    }

    async fn respawn_lease_keeper(&mut self, context: &mut ActorContext) {
        match self.spawn_lease_keeper(context).await {
            Ok(lease_id) => {
                self.lease_id = Some(lease_id);
                if let Some(error) = self.put_self_member().await.err() {
                    error!("{} put self member error {:?}", context.myself(), error);
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
struct WatchResp(EWatchResp);

#[async_trait]
impl Message for WatchResp {
    type A = ClusterDaemon;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let EWatchResp { key, resp } = self.0;
        if key == actor.node_path() {
            Self::update_cluster_node(context, actor, &resp)?;
        } else if key == actor.lease_path() {
            Self::update_member_status(context, actor, resp)?;
        }
        Ok(())
    }
}

impl WatchResp {
    fn update_cluster_node(context: &mut ActorContext, actor: &mut ClusterDaemon, resp: &WatchResponse) -> anyhow::Result<()> {
        for event in resp.events() {
            if matches!(event.event_type(), EventType::Put) {
                if let Some(kv) = event.kv() {
                    let new_cluster_node = serde_json::from_slice::<ClusterNode>(kv.value())?;
                    let cluster = Cluster::get(context.system());
                    let node = cluster.node().write().unwrap();
                    let removed_socket_addrs = node.addrs.sub(&new_cluster_node.addrs);
                    let mut removed_addrs = vec![];
                    let mut removed_members = vec![];
                    *cluster.node().write().unwrap() = new_cluster_node;
                    let mut state = cluster.state().write().unwrap();
                    for addr in state.members.keys().chain(state.unreachable.keys()) {
                        let socket_addr = addr.socket_addr().unwrap();
                        if removed_socket_addrs.contains(socket_addr) {
                            removed_addrs.push(addr.clone());
                        }
                    }
                    for addr in removed_addrs {
                        if let Some(member) = state.members.remove(&addr) {
                            removed_members.push(member);
                        }
                        if let Some(member) = state.unreachable.remove(&addr) {
                            removed_members.push(member);
                        }
                    }
                    let stream = context.system().event_stream();
                    for member in removed_members {
                        if member.addr == actor.self_addr {
                            let cluster = Cluster::get(context.system());
                            actor.self_removed(cluster.state().write().unwrap(), cluster.self_member().write().unwrap());
                        }
                        let event = ClusterEvent::MemberEvent(MemberEvent::MemberRemoved(member));
                        stream.publish(DynMessage::orphan(event))?;
                    }
                }
            }
        }
        Ok(())
    }

    fn update_member_status(context: &mut ActorContext, actor: &mut ClusterDaemon, resp: WatchResponse) -> anyhow::Result<()> {
        for event in resp.events() {
            if let Some(kv) = event.kv() {
                let cluster = Cluster::get(context.system());
                let mut state = cluster.state().write().unwrap();
                match event.event_type() {
                    EventType::Put => {
                        let member = serde_json::from_slice::<Member>(kv.value())?;
                        actor.key_addr.insert(kv.key_str()?.to_string(), member.addr.clone());
                        debug!("{} put member {:?}", context.myself(), member);
                        if member.addr == actor.self_addr {
                            *cluster.self_member().write().unwrap() = member.clone();
                        }
                        if !matches!(member.status, MemberStatus::Removed) {
                            state.unreachable.remove(&member.addr);
                            state.members.insert(member.addr.clone(), member.clone());
                        } else {
                            // member down
                            state.members.remove(&member.addr);
                            state.unreachable.insert(member.addr.clone(), member.clone());
                            if member.addr == actor.self_addr {
                                actor.self_removed(state, cluster.self_member().write().unwrap());
                            }
                        }
                        let event = match member.status {
                            MemberStatus::Joining => {
                                ClusterEvent::MemberEvent(MemberEvent::MemberJoined(member))
                            }
                            MemberStatus::Up => {
                                ClusterEvent::MemberEvent(MemberEvent::MemberUp(member))
                            }
                            MemberStatus::Leaving => {
                                ClusterEvent::MemberEvent(MemberEvent::MemberLeft(member))
                            }
                            MemberStatus::Exiting => {
                                ClusterEvent::MemberEvent(MemberEvent::MemberExited(member))
                            }
                            MemberStatus::Down => {
                                ClusterEvent::MemberEvent(MemberEvent::MemberDowned(member))
                            }
                            MemberStatus::Removed => {
                                ClusterEvent::MemberEvent(MemberEvent::MemberRemoved(member))
                            }
                        };
                        context.system().event_stream().publish(DynMessage::orphan(event))?;
                    }
                    EventType::Delete => {
                        if let Some(addr) = actor.key_addr.remove(kv.key_str()?) {
                            if let Some(mut member) = state.members.remove(&addr) {
                                member.status = MemberStatus::Down;
                                debug!("{} delete member {:?}", context.myself(), addr);
                                state.unreachable.insert(addr.clone(), member.clone());
                                if addr == actor.self_addr {
                                    actor.self_removed(state, cluster.self_member().write().unwrap());
                                }
                                let event = ClusterEvent::MemberEvent(MemberEvent::MemberDowned(member));
                                context.system().event_stream().publish(DynMessage::orphan(event))?;
                            }
                        }
                    }
                }
            }
        }
        Ok(())
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

#[tokio::test]
async fn test() -> anyhow::Result<()> {
    let mut nodes = BTreeSet::new();
    nodes.insert("127.0.0.1:12121".parse()?);
    nodes.insert("127.0.0.1:12123".parse()?);
    let n = ClusterNode { addrs: nodes };
    let r = serde_json::to_vec(&n).unwrap();
    let mut client = Client::connect(["localhost:2379"], None).await?;
    client.put("actor/mikai233/cluster/node", r, None).await?;
    Ok(())
}
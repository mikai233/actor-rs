use std::collections::{BTreeSet, HashSet};
use std::collections::hash_map::Entry;
use std::fmt::{Debug, Formatter};
use std::net::SocketAddrV4;
use std::time::Duration;

use async_trait::async_trait;
use etcd_client::{Client, EventType, PutOptions, WatchOptions};
use tracing::{debug, trace};

use actor_core::{Actor, DynMessage, Message};
use actor_core::actor::actor_ref::ActorRef;
use actor_core::actor::actor_ref_factory::ActorRefFactory;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::props::Props;
use actor_core::ext::option_ext::OptionExt;
use actor_derive::EmptyCodec;

use crate::cluster::Cluster;
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
}

impl Debug for ClusterDaemon {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_struct("ClusterDaemon")
            .field("eclient", &"..")
            .field("self_addr", &self.self_addr)
            .field("roles", &self.roles)
            .field("transport", &self.transport)
            .field("lease_id", &self.lease_id)
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
        Self::spawn_watcher(context, "node_watcher", adapter.clone(), self.node_path(), None, self.eclient.clone())?;
        Self::spawn_watcher(
            context,
            "lease_watcher",
            adapter.clone(),
            self.lease_path(),
            Some(WatchOptions::new().with_prefix()),
            self.eclient.clone(),
        )?;
        if self.is_addr_in_cluster(context, self.self_addr.address.addr.as_ref().unwrap()) {
            let lease_id = self.spawn_lease_keeper(context).await?;
            self.lease_id = Some(lease_id);
            self.put_self_member().await?;
        }
        Ok(())
    }
}

impl ClusterDaemon {
    fn node_path(&self) -> String {
        format!("actor/{}/cluster/node", self.self_addr.system_name())
    }

    fn lease_path(&self) -> String {
        format!("actor/{}/cluster/lease/", self.self_addr.system_name())
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
        cluster_node.nodes.contains(addr)
    }

    async fn put_self_member(&mut self) -> anyhow::Result<()> {
        let current_lease = format!("{}{}", self.lease_path(), self.self_addr.socket_addr().unwrap());
        let member = Member::new(self.self_addr.clone(), MemberStatus::Up, self.roles.clone());
        let member_json = serde_json::to_vec(&member)?;
        let put_options = PutOptions::new().with_lease(self.lease_id.unwrap());
        self.eclient.put(current_lease, member_json, Some(put_options)).await?;
        Ok(())
    }

    fn update_member(&mut self, context: &mut ActorContext, member: Member) {
        let cluster = context.system().get_extension::<Cluster>().unwrap();
        let mut state = cluster.state().write().unwrap();
        match state.members.entry(member.addr.address.clone()) {
            Entry::Occupied(mut o) => {
                if o.get().addr != member.addr {
                    //TODO member remove event
                }
                o.insert(member);
                //TODO status change ?
            }
            Entry::Vacant(v) => {
                //TODO member add event
                v.insert(member);
            }
        }
    }

    fn remove_member(&mut self, context: &mut ActorContext, member: Member) {
        let cluster = Cluster::get(context.system());
        let state = cluster.state().write().unwrap();
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
            for event in resp.events() {
                if matches!(event.event_type(), EventType::Put) {
                    if let Some(kv) = event.kv() {
                        let cluster_node = serde_json::from_slice::<ClusterNode>(kv.value())?;
                        let cluster = Cluster::get(context.system());
                        *cluster.node().write().unwrap() = cluster_node;
                    }
                }
            }
        } else if key == actor.lease_path() {
            for event in resp.events() {
                match event.event_type() {
                    EventType::Put => {
                        if let Some(kv) = event.kv() {
                            let member = serde_json::from_slice::<Member>(kv.value())?;
                            let cluster = Cluster::get(context.system());
                            if member.addr == actor.self_addr {
                                *cluster.self_member().write().unwrap() = member.clone();
                            }
                            debug!("{} put member {:?}", context.myself(), member);
                            cluster.state().write().unwrap().members.insert(member.addr.address.clone(), member);
                        }
                    }
                    EventType::Delete => {
                        debug!("{} delete member", context.myself());
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
        Ok(())
    }
}

#[tokio::test]
async fn test() -> anyhow::Result<()> {
    let mut nodes = BTreeSet::new();
    nodes.insert("127.0.0.1:12121".parse()?);
    nodes.insert("127.0.0.1:12123".parse()?);
    let n = ClusterNode { nodes };
    let r = serde_json::to_vec(&n).unwrap();
    let mut client = Client::connect(["localhost:2379"], None).await?;
    client.put("actor/mikai233/cluster/node", r, None).await?;
    Ok(())
}
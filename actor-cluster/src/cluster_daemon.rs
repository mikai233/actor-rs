use std::collections::hash_map::Entry;
use std::collections::HashSet;
use std::fmt::{Debug, Formatter};
use std::net::SocketAddrV4;
use std::time::Duration;

use async_recursion::async_recursion;
use async_trait::async_trait;
use etcd_client::{Client, EventType, WatchOptions};
use tracing::{error, trace};

use actor_core::{Actor, DynMessage, Message};
use actor_core::actor::actor_ref::ActorRef;
use actor_core::actor::actor_ref_factory::ActorRefFactory;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::props::Props;
use actor_derive::EmptyCodec;

use crate::cluster::Cluster;
use crate::cluster_heartbeat::{ClusterHeartbeatReceiver, ClusterHeartbeatSender};
use crate::cluster_node::ClusterNode;
use crate::ewatcher::{EWatcher, EWatchResp};
use crate::lease_keeper::{LeaseKeepAliveFailed, LeaseKeeper};
use crate::member::Member;
use crate::unique_address::UniqueAddress;

pub struct ClusterDaemon {
    pub(crate) eclient: Client,
    pub(crate) self_unique_address: UniqueAddress,
    pub(crate) roles: HashSet<String>,
    pub(crate) transport: ActorRef,
    pub(crate) lease_id: Option<i64>,
}

impl Debug for ClusterDaemon {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_struct("ClusterDaemon")
            .field("eclient", &"..")
            .field("self_unique_address", &self.self_unique_address)
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
        let cluster_node = self.get_cluster_node().await;
        let cluster = Cluster::get(context.system());
        *cluster.node().write().unwrap() = cluster_node;
        drop(cluster);
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
        if self.is_addr_in_cluster(context, self.self_unique_address.address.addr.as_ref().unwrap()) {
            let lease_id = self.spawn_lease_keeper(context).await?;
            self.lease_id = Some(lease_id);
        }
        Ok(())
    }
}

impl ClusterDaemon {
    fn node_path(&self) -> String {
        format!("actor/{}/cluster/node", self.self_unique_address.address.system)
    }

    fn lease_path(&self) -> String {
        format!("actor/{}/cluster/lease/", self.self_unique_address.address.system)
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

    #[async_recursion]
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
    #[async_recursion]
    async fn get_cluster_node(&mut self) -> ClusterNode {
        let node_path = self.node_path();
        match self.eclient.get(node_path.as_bytes(), None).await {
            Ok(mut rsp) => {
                let kv = rsp.kvs().first().expect(&*format!("{} not found in etcd", node_path));
                match serde_json::from_slice::<ClusterNode>(kv.value()) {
                    Ok(cluster_node) => cluster_node,
                    Err(error) => {
                        //TODO graceful shutdown
                        panic!("invalid json format in path {} {:?}", node_path, error);
                    }
                }
            }
            Err(error) => {
                let addr = &self.self_unique_address;
                error!("{} unable to get node info form etcd {:?}, retry after 3s", addr, error);
                tokio::time::sleep(Duration::from_secs(3)).await;
                self.get_cluster_node().await
            }
        }
    }

    fn is_addr_in_cluster(&self, context: &mut ActorContext, addr: &SocketAddrV4) -> bool {
        let cluster = Cluster::get(context.system());
        let cluster_node = cluster.node().read().unwrap();
        cluster_node.nodes.contains(addr)
    }

    fn update_member(&mut self, context: &mut ActorContext, member: Member) {
        let cluster = context.system().get_extension::<Cluster>().unwrap();
        let mut state = cluster.state().write().unwrap();
        match state.members.entry(member.unique_address.address.clone()) {
            Entry::Occupied(mut o) => {
                if o.get().unique_address != member.unique_address {
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
                        match serde_json::from_slice::<ClusterNode>(kv.value()) {
                            Ok(cluster_node) => {
                                let cluster = Cluster::get(context.system());
                                *cluster.node().write().unwrap() = cluster_node;
                            }
                            Err(error) => {
                                error!("invalid json format in path {} {:?}", key, error);
                            }
                        }
                    }
                }
            }
        } else if key == actor.lease_path() {}
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
    let n = ClusterNode {
        nodes: vec!["127.0.0.1:12121".parse().unwrap(), "127.0.0.1:12123".parse().unwrap()],
    };
    let r = serde_json::to_vec(&n).unwrap();
    let mut client = Client::connect(["localhost:2379"], None).await?;
    client.put("actor/mikai233/cluster/node", r, None).await?;
    Ok(())
}
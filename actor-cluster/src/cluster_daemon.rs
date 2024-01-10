use std::collections::hash_map::Entry;
use std::collections::HashSet;
use std::fmt::{Debug, Formatter};
use std::net::SocketAddrV4;
use std::time::Duration;

use async_recursion::async_recursion;
use async_trait::async_trait;
use etcd_client::{Client, EventType, GetOptions, WatchOptions};
use tracing::{debug, error, info, trace};

use actor_core::{Actor, DynMessage, Message};
use actor_core::actor::actor_ref::{ActorRef, ActorRefExt};
use actor_core::actor::actor_ref_factory::ActorRefFactory;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::props::Props;
use actor_derive::EmptyCodec;
use actor_remote::net::message::Connect;

use crate::cluster::Cluster;
use crate::cluster_heartbeat::{ClusterHeartbeatReceiver, ClusterHeartbeatSender};
use crate::ewatcher::{EWatcher, EWatchResp};
use crate::member::Member;
use crate::unique_address::UniqueAddress;

pub struct ClusterDaemon {
    pub(crate) eclient: Client,
    pub(crate) self_unique_address: UniqueAddress,
    pub(crate) roles: HashSet<String>,
    pub(crate) transport: ActorRef,
}

impl Debug for ClusterDaemon {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_struct("ClusterDaemon")
            .field("eclient", &"..")
            .field("self_unique_address", &self.self_unique_address)
            .field("roles", &self.roles)
            .field("transport", &self.transport)
            .finish()
    }
}

#[async_trait]
impl Actor for ClusterDaemon {
    async fn pre_start(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        trace!("{} started", context.myself());
        let heartbeat_sender = context.spawn_actor(Props::create(|_| ClusterHeartbeatSender::default()), ClusterHeartbeatSender::name())?;
        let heartbeat_receiver = context.spawn_actor(Props::create(|_| ClusterHeartbeatReceiver::default()), ClusterHeartbeatReceiver::name())?;
        let addrs = self.get_cluster_nodes().await;
        for addr in addrs {
            debug!("{} try to connect {}", context.myself(), addr);
            self.transport.cast_ns(Connect::with_infinite_retry(addr.into()));
            //TODO heartbeat
        }
        let adapter = context.message_adapter::<EWatchResp>(|m| {
            let m = DynMessage::user(WatchResp(m));
            Ok(m)
        });
        Self::spawn_watcher(context, "node_watcher", adapter.clone(), self.node_path(), self.eclient.clone())?;
        Self::spawn_watcher(context, "lease_watcher", adapter.clone(), self.lease_path(), self.eclient.clone())?;
        Ok(())
    }
}

impl ClusterDaemon {
    fn node_path(&self) -> String {
        format!("actor/{}/cluster/node/", self.self_unique_address.address.system)
    }

    fn lease_path(&self) -> String {
        format!("actor/{}/cluster/lease/", self.self_unique_address.address.system)
    }

    fn spawn_watcher(context: &mut ActorContext, name: impl Into<String>, adapter: ActorRef, key: String, eclient: Client) -> anyhow::Result<()> {
        context.spawn_actor(Props::create(move |ctx| {
            EWatcher::new(
                ctx.myself().clone(),
                eclient.clone(),
                key.clone(),
                Some(WatchOptions::new().with_prefix()),
                adapter.clone(),
            )
        }), name.into())?;
        Ok(())
    }

    #[async_recursion]
    async fn get_cluster_nodes(&mut self) -> Vec<SocketAddrV4> {
        let node_path = self.node_path();
        match self.eclient.get(node_path.as_bytes(), Some(GetOptions::new().with_prefix())).await {
            Ok(rsp) => {
                let mut addrs = vec![];
                for kv in rsp.kvs() {
                    match kv.key_str() {
                        Ok(key) => {
                            match key.strip_prefix(&node_path) {
                                None => {
                                    error!("incorrect node addr in key {}", key);
                                }
                                Some(addr) => {
                                    match addr.parse::<SocketAddrV4>() {
                                        Ok(addr) => {
                                            addrs.push(addr);
                                        }
                                        Err(error) => {
                                            error!("incorrect node addr in key {} {:?}", key, error);
                                        }
                                    }
                                }
                            }
                        }
                        Err(error) => {
                            error!("unexpected key in path {} {:?}", node_path, error);
                        }
                    }
                }
                addrs
            }
            Err(error) => {
                let addr = &self.self_unique_address;
                error!("{} unable to get node info form etcd {:?}, retry after 3s", addr, error);
                tokio::time::sleep(Duration::from_secs(3)).await;
                self.get_cluster_nodes().await
            }
        }
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
        //TODO Cluster::get ?
        let cluster = context.system().get_extension::<Cluster>().unwrap();
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
                let key = event.kv();
                info!("{:?}", key);
                match event.event_type() {
                    EventType::Put => {}
                    EventType::Delete => {}
                }
            }
        } else if key == actor.lease_path() {}
        Ok(())
    }
}
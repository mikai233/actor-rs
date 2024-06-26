use std::any::type_name;
use std::fmt::Debug;

use ahash::HashSet;
use tokio::sync::broadcast::Receiver;

use actor_core::actor::actor_system::ActorSystem;
use actor_core::actor::address::Address;
use actor_core::actor::props::{DeferredSpawn, FuncDeferredSpawn, Props};
use actor_core::actor_path::ActorPath;
use actor_core::actor_ref::ActorRef;
use actor_core::actor_ref::local_ref::LocalActorRef;
use actor_core::AsAny;
use actor_core::ext::etcd_client::EtcdClient;
use actor_core::message::message_registry::MessageRegistry;
use actor_core::provider::{ActorRefProvider, TActorRefProvider};
use actor_core::provider::local_actor_ref_provider::LocalActorRefProvider;
use actor_remote::remote_provider::RemoteActorRefProvider;
use actor_remote::remote_setting::RemoteSetting;

use crate::cluster::Cluster;
use crate::cluster_setting::ClusterSetting;
use crate::heartbeat::cluster_heartbeat_receiver::heartbeat::Heartbeat;
use crate::heartbeat::cluster_heartbeat_sender::heartbeat_rsp::HeartbeatRsp;

#[derive(Debug, AsAny)]
pub struct ClusterActorRefProvider {
    pub remote: RemoteActorRefProvider,
    pub client: EtcdClient,
    pub roles: HashSet<String>,
}

impl ClusterActorRefProvider {
    pub fn new(system: ActorSystem, setting: ClusterSetting) -> anyhow::Result<(Self, Vec<Box<dyn DeferredSpawn>>)> {
        let ClusterSetting { config: cluster_config, mut reg, client } = setting;
        let remote_config = cluster_config.remote.clone();
        let roles = cluster_config.roles.clone();
        system.add_config(cluster_config)?;
        Self::register_system_message(&mut reg);
        let remote_setting = RemoteSetting { config: remote_config, reg };
        let (remote, mut spawns) = RemoteActorRefProvider::new(system, remote_setting)?;
        let cluster = ClusterActorRefProvider {
            remote,
            client,
            roles,
        };
        Self::register_extension(&mut spawns);
        Ok((cluster, spawns))
    }

    fn register_extension(spawns: &mut Vec<Box<dyn DeferredSpawn>>) {
        let s = FuncDeferredSpawn::new(|system| {
            system.register_extension(|system| Cluster::new(system))?;
            Ok(())
        });
        spawns.push(Box::new(s));
    }

    fn register_system_message(reg: &mut MessageRegistry) {
        reg.register_system::<Heartbeat>();
        reg.register_system::<HeartbeatRsp>();
    }

    pub fn builder(cluster_setting: ClusterSetting) -> impl Fn(ActorSystem) -> anyhow::Result<(ActorRefProvider, Vec<Box<dyn DeferredSpawn>>)> {
        move |system: ActorSystem| {
            Self::new(system, cluster_setting.clone()).map(|t| { (t.0.into(), t.1) })
        }
    }
}

impl TActorRefProvider for ClusterActorRefProvider {
    fn root_guardian(&self) -> &LocalActorRef {
        self.remote.root_guardian()
    }

    fn root_guardian_at(&self, address: &Address) -> ActorRef {
        self.remote.root_guardian_at(address)
    }

    fn guardian(&self) -> &LocalActorRef {
        self.remote.guardian()
    }

    fn system_guardian(&self) -> &LocalActorRef {
        self.remote.system_guardian()
    }

    fn root_path(&self) -> &ActorPath {
        self.remote.root_path()
    }

    fn temp_path(&self) -> ActorPath {
        self.remote.temp_path()
    }

    fn temp_path_of_prefix(&self, prefix: Option<&String>) -> ActorPath {
        self.remote.temp_path_of_prefix(prefix)
    }

    fn temp_container(&self) -> ActorRef {
        self.remote.temp_container()
    }

    fn register_temp_actor(&self, actor: ActorRef, path: &ActorPath) {
        self.remote.register_temp_actor(actor, path)
    }

    fn unregister_temp_actor(&self, path: &ActorPath) {
        self.remote.unregister_temp_actor(path)
    }

    fn spawn_actor(&self, props: Props, supervisor: &ActorRef) -> anyhow::Result<ActorRef> {
        self.remote.spawn_actor(props, supervisor)
    }

    fn resolve_actor_ref_of_path(&self, path: &ActorPath) -> ActorRef {
        self.remote.resolve_actor_ref_of_path(path)
    }

    fn dead_letters(&self) -> &ActorRef {
        self.remote.dead_letters()
    }

    fn ignore_ref(&self) -> &ActorRef {
        self.remote.ignore_ref()
    }

    fn termination_rx(&self) -> Receiver<()> {
        self.remote.termination_rx()
    }

    fn as_provider(&self, name: &str) -> Option<&dyn TActorRefProvider> {
        if name == type_name::<Self>() {
            Some(self)
        } else if name == type_name::<RemoteActorRefProvider>() {
            Some(&self.remote)
        } else if name == type_name::<LocalActorRefProvider>() {
            Some(&self.remote.local)
        } else {
            None
        }
    }
}

impl Into<ActorRefProvider> for ClusterActorRefProvider {
    fn into(self) -> ActorRefProvider {
        ActorRefProvider::new(self)
    }
}
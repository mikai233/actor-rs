use std::collections::HashSet;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;

use anyhow::Context;
use tokio::sync::broadcast::Receiver;

use actor_core::actor::actor_path::ActorPath;
use actor_core::actor::actor_ref::ActorRef;
use actor_core::actor::actor_ref_provider::{ActorRefProvider, TActorRefProvider};
use actor_core::actor::address::Address;
use actor_core::actor::local_ref::LocalActorRef;
use actor_core::actor::props::{DeferredSpawn, FuncDeferredSpawn, Props};
use actor_core::config::Config;
use actor_core::message::message_registration::MessageRegistration;
use actor_derive::AsAny;
use actor_remote::remote_provider::RemoteActorRefProvider;
use actor_remote::remote_setting::RemoteSetting;

use crate::{CLUSTER_CONFIG, CLUSTER_CONFIG_NAME};
use crate::cluster::Cluster;
use crate::cluster_heartbeat::{Heartbeat, HeartbeatRsp};
use crate::cluster_setting::ClusterSetting;
use crate::config::ClusterConfig;
use crate::distributed_pub_sub::DistributedPubSub;

#[derive(AsAny)]
pub struct ClusterActorRefProvider {
    pub remote: RemoteActorRefProvider,
    pub eclient: etcd_client::Client,
    pub roles: HashSet<String>,
}

impl Debug for ClusterActorRefProvider {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_struct("ClusterActorRefProvider")
            .field("remote", &self.remote)
            .field("eclient", &"..")
            .finish()
    }
}

impl ClusterActorRefProvider {
    pub fn new(setting: ClusterSetting) -> anyhow::Result<(Self, Vec<Box<dyn DeferredSpawn>>)> {
        let ClusterSetting { system, config, mut reg, eclient } = setting;
        let default_config: ClusterConfig = toml::from_str(CLUSTER_CONFIG).context(format!("failed to load {}", CLUSTER_CONFIG_NAME))?;
        let cluster_config = config.with_fallback(default_config);
        let remote_config = cluster_config.remote.clone();
        let roles = cluster_config.roles.clone();
        system.add_config(cluster_config)?;
        Self::register_system_message(&mut reg);
        let remote_setting = RemoteSetting::builder()
            .reg(reg)
            .config(remote_config)
            .system(system.clone())
            .build();
        let (remote, mut spawns) = RemoteActorRefProvider::new(remote_setting)?;
        let cluster = ClusterActorRefProvider {
            remote,
            eclient,
            roles,
        };
        Self::register_extension(&mut spawns);
        Ok((cluster, spawns))
    }

    fn register_extension(spawns: &mut Vec<Box<dyn DeferredSpawn>>) {
        let s = FuncDeferredSpawn::new(|system| {
            system.register_extension(|system| Cluster::new(system))?;
            system.register_extension(|system| DistributedPubSub::new(system))?;
            Ok(())
        });
        spawns.push(Box::new(s));
    }

    fn register_system_message(reg: &mut MessageRegistration) {
        reg.register_system::<Heartbeat>();
        reg.register_system::<HeartbeatRsp>();
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

    fn registration(&self) -> Option<&Arc<MessageRegistration>> {
        self.remote.registration()
    }

    fn termination_rx(&self) -> Receiver<()> {
        self.remote.termination_rx()
    }
}

impl Into<ActorRefProvider> for ClusterActorRefProvider {
    fn into(self) -> ActorRefProvider {
        ActorRefProvider::new(self)
    }
}
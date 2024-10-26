use std::any::type_name;
use std::fmt::Debug;

use actor_core::provider::provider::Provider;
use actor_remote::codec::MessageCodecRegistry;
use actor_remote::register_remote_message;
use ahash::HashSet;
use config::Config;
use tokio::sync::broadcast::Receiver;

use actor_core::actor::actor_system::ActorSystem;
use actor_core::actor::address::Address;
use actor_core::actor::props::Props;
use actor_core::actor_path::ActorPath;
use actor_core::actor_ref::local_ref::LocalActorRef;
use actor_core::actor_ref::{ActorRef, TActorRef};
use actor_core::provider::local_provider::LocalActorRefProvider;
use actor_core::provider::{ActorRefProvider, TActorRefProvider};
use actor_core::AsAny;
use actor_remote::remote_provider::RemoteActorRefProvider;

use crate::cluster::Cluster;
use crate::config::settings::Settings;

#[derive(Debug, AsAny)]
pub struct ClusterActorRefProvider {
    pub settings: Settings,
    pub remote: RemoteActorRefProvider,
    pub roles: HashSet<String>,
}

impl ClusterActorRefProvider {
    pub fn new<R>(
        system: ActorSystem,
        config: &Config,
        mut registry: R,
    ) -> anyhow::Result<Provider<Self>>
    where
        R: MessageCodecRegistry,
    {
        let settings = Settings::new(config)?;
        register_remote_message(&mut registry);
        let Provider {
            name,
            provider,
            spawns,
        } = RemoteActorRefProvider::new(system, config, registry)?;
        let cluster = Self {
            settings,
            remote: provider.provider,
            roles: Default::default(),
        };
        let cluster = Cluster::new(system)?;
        system.register_extension(cluster)?;
        Ok(Provider::new(cluster, provider.spawns))
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

    fn temp_path_of_prefix(&self, prefix: Option<&str>) -> ActorPath {
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

    fn spawn_actor(
        &self,
        props: Props,
        supervisor: &ActorRef,
        system: ActorSystem,
    ) -> anyhow::Result<ActorRef> {
        self.remote.spawn_actor(props, supervisor, system)
    }

    fn resolve_actor_ref_of_path(&self, path: &ActorPath) -> ActorRef {
        self.remote.resolve_actor_ref_of_path(path)
    }

    fn dead_letters(&self) -> &dyn TActorRef {
        self.remote.dead_letters()
    }

    fn ignore_ref(&self) -> &dyn TActorRef {
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

    fn config(&self) -> &config::Config {
        &self.remote.local.config
    }
}

impl Into<ActorRefProvider> for ClusterActorRefProvider {
    fn into(self) -> ActorRefProvider {
        ActorRefProvider::new(self)
    }
}

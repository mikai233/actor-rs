use std::any::type_name;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use config::{Config, File, FileFormat};
use tokio::sync::broadcast::Receiver;

use crate::codec::MessageRegistry;
use actor_core::actor::actor_system::ActorSystem;
use actor_core::actor::address::{Address, Protocol};
use actor_core::actor::props::{ActorDeferredSpawn, Props};
use actor_core::actor_path::root_actor_path::RootActorPath;
use actor_core::actor_path::ActorPath;
use actor_core::actor_path::TActorPath;
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::actor_ref::local_ref::LocalActorRef;
use actor_core::actor_ref::ActorRef;
use actor_core::provider::builder::{Provider, ProviderBuilder};
use actor_core::provider::local_provider::LocalActorRefProvider;
use actor_core::provider::{ActorRefProvider, TActorRefProvider};
use actor_core::AsAny;

use crate::artery::ArteryActor;
use crate::config::advanced::Advanced;
use crate::config::artery::Transport;
use crate::config::settings::Settings;
use crate::failure_detector::default_failure_detector_registry::DefaultFailureDetectorRegistry;
use crate::failure_detector::phi_accrual_failure_detector::PhiAccrualFailureDetector;
use crate::remote_actor_ref::RemoteActorRef;
use crate::remote_watcher::artery_heartbeat::ArteryHeartbeat;
use crate::remote_watcher::artery_heartbeat_rsp::ArteryHeartbeatRsp;
use crate::remote_watcher::heartbeat::Heartbeat;
use crate::remote_watcher::heartbeat_rsp::HeartbeatRsp;
use crate::remote_watcher::RemoteWatcher;

#[derive(Debug, AsAny)]
pub struct RemoteActorRefProvider {
    pub settings: Settings,
    pub local: LocalActorRefProvider,
    pub address: Address,
    pub artery: ActorRef,
    pub registry: Arc<MessageRegistry>,
    pub remote_watcher: ActorRef,
}

impl RemoteActorRefProvider {
    pub fn new(system: ActorSystem, config: &Config, mut registry: MessageRegistry) -> anyhow::Result<Provider<Self>> {
        Self::register_system_message(&mut registry);
        let settings = Settings::new(config)?;
        let canonical = settings.artery.canonical;
        let transport = settings.artery.transport;
        let address = Address::new(Protocol::Akka, system.name.clone(), Some(canonical));
        let mut provider = LocalActorRefProvider::new(system, config, Some(address.clone()))?;
        let (artery, deferred) = RemoteActorRefProvider::spawn_artery(&provider.provider, transport, canonical, settings.advanced.clone())?;
        provider.spawns.push(Box::new(deferred));
        let (remote_watcher, remote_watcher_deferred) = Self::create_remote_watcher(&provider.provider)?;
        provider.spawns.push(Box::new(remote_watcher_deferred));
        let remote = Self {
            settings,
            local: provider.provider,
            address,
            artery,
            registry: Arc::new(registry),
            remote_watcher,
        };
        Ok(Provider::new(remote, provider.spawns))
    }

    pub(crate) fn spawn_artery(
        provider: &LocalActorRefProvider,
        transport: Transport,
        socket_addr: SocketAddr,
        advanced: Advanced,
    ) -> anyhow::Result<(ActorRef, ActorDeferredSpawn)> {
        provider.system_guardian()
            .attach_child_deferred_start(
                Props::new_with_ctx(
                    move |context| {
                        ArteryActor::new(context.system().clone(), transport, socket_addr, advanced)
                    },
                ),
                Some("artery".to_string()),
                None,
            )
    }

    fn has_address(&self, address: &Address) -> bool {
        address == self.local.root_path().address() || address == self.root_path().address() || address == &self.address
    }

    fn create_remote_watcher(provider: &LocalActorRefProvider) -> anyhow::Result<(ActorRef, ActorDeferredSpawn)> {
        provider.system_guardian()
            .attach_child_deferred_start(
                RemoteWatcher::props(Self::create_remote_watcher_failure_detector()),
                Some("remote_watcher".to_string()),
                None,
            )
    }

    fn create_remote_watcher_failure_detector() -> DefaultFailureDetectorRegistry<Address> {
        DefaultFailureDetectorRegistry::new(|| {
            let detector = PhiAccrualFailureDetector::new(
                10.0,
                200,
                Duration::from_millis(100),
                Duration::from_secs(10),
                Duration::from_secs(1),
            );
            Box::new(detector)
        })
    }

    fn register_system_message(reg: &mut MessageRegistry) {
        reg.register_system::<ArteryHeartbeat>();
        reg.register_system::<ArteryHeartbeatRsp>();
        reg.register_system::<Heartbeat>();
        reg.register_system::<HeartbeatRsp>();
    }
}

impl TActorRefProvider for RemoteActorRefProvider {
    fn root_guardian(&self) -> &LocalActorRef {
        self.local.root_guardian()
    }

    fn root_guardian_at(&self, address: &Address) -> ActorRef {
        if self.has_address(address) {
            self.root_guardian().clone().into()
        } else {
            let remote = RemoteActorRef::new(
                self.guardian().system().clone(),
                RootActorPath::new(address.clone(), "/").into(),
                self.artery.clone(),
                self.remote_watcher.clone(),
            );
            remote.into()
        }
    }

    fn guardian(&self) -> &LocalActorRef {
        self.local.guardian()
    }

    fn system_guardian(&self) -> &LocalActorRef {
        self.local.system_guardian()
    }

    fn root_path(&self) -> &ActorPath {
        self.local.root_path()
    }

    fn temp_path(&self) -> ActorPath {
        self.local.temp_path()
    }

    fn temp_path_of_prefix(&self, prefix: Option<&str>) -> ActorPath {
        self.local.temp_path_of_prefix(prefix)
    }

    fn temp_container(&self) -> ActorRef {
        self.local.temp_container()
    }

    fn register_temp_actor(&self, actor: ActorRef, path: &ActorPath) {
        self.local.register_temp_actor(actor, path)
    }

    fn unregister_temp_actor(&self, path: &ActorPath) {
        self.local.unregister_temp_actor(path)
    }

    fn spawn_actor(&self, props: Props, supervisor: &ActorRef, system: ActorSystem) -> anyhow::Result<ActorRef> {
        // TODO remote spawn
        self.local.spawn_actor(props, supervisor)
    }

    fn resolve_actor_ref_of_path(&self, path: &ActorPath) -> ActorRef {
        if path.address() == self.root_path().address() {
            self.local.resolve_actor_ref_of_path(path)
        } else {
            let remote = RemoteActorRef::new(
                self.artery.system().clone(),
                path.clone(),
                self.artery.clone(),
                self.remote_watcher.clone(),
            );
            remote.into()
        }
    }

    fn dead_letters(&self) -> &ActorRef {
        &self.local.dead_letters()
    }

    fn ignore_ref(&self) -> &ActorRef {
        self.local.ignore_ref()
    }

    fn termination_rx(&self) -> Receiver<()> {
        self.local.termination_rx()
    }

    fn as_provider(&self, name: &str) -> Option<&dyn TActorRefProvider> {
        if name == type_name::<Self>() {
            Some(self)
        } else if name == type_name::<LocalActorRefProvider>() {
            Some(&self.local)
        } else {
            None
        }
    }
}

impl ProviderBuilder<Self> for RemoteActorRefProvider {
    fn build(system: ActorSystem, config: Config, registry: MessageRegistry) -> anyhow::Result<Provider<Self>> {
        let config = Config::builder()
            .add_source(File::from_str(actor_core::REFERENCE, FileFormat::Toml))
            .add_source(File::from_str(crate::REFERENCE, FileFormat::Toml))
            .add_source(config)
            .build()?;
        Self::new(system, &config, registry)
    }
}

impl Into<ActorRefProvider> for RemoteActorRefProvider {
    fn into(self) -> ActorRefProvider {
        ActorRefProvider::new(self)
    }
}
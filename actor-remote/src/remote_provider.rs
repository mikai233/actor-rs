use std::any::type_name;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use tokio::sync::broadcast::Receiver;

use crate::artery::ArteryActor;
use crate::codec::{register_remote_system_message, MessageCodecRegistry};
use crate::config::advanced::Advanced;
use crate::config::artery::Transport;
use crate::config::settings::Remote;
use crate::failure_detector::default_failure_detector_registry::DefaultFailureDetectorRegistry;
use crate::failure_detector::phi_accrual_failure_detector::PhiAccrualFailureDetector;
use crate::remote_actor_ref::RemoteActorRef;
use crate::remote_watcher::RemoteWatcher;
use actor_core::actor::actor_system::ActorSystem;
use actor_core::actor::address::{Address, Protocol};
use actor_core::actor::props::Props;
use actor_core::actor_path::root_actor_path::RootActorPath;
use actor_core::actor_path::ActorPath;
use actor_core::actor_path::TActorPath;
use actor_core::actor_ref::local_ref::LocalActorRef;
use actor_core::actor_ref::{ActorRef, TActorRef};
use actor_core::provider::local_provider::LocalActorRefProvider;
use actor_core::provider::provider::{ActorSpawn, Provider};
use actor_core::provider::{ActorRefProvider, TActorRefProvider};
use actor_core::AsAny;

#[derive(Debug, AsAny)]
pub struct RemoteActorRefProvider {
    pub remote_cfg: Remote,
    pub local: LocalActorRefProvider,
    pub address: Address,
    pub artery: ActorRef,
    pub registry: Arc<dyn MessageCodecRegistry>,
    pub remote_watcher: ActorRef,
}

impl RemoteActorRefProvider {
    pub fn new<R>(
        name: impl Into<String>,
        config: Option<config::Config>,
        mut registry: R,
    ) -> anyhow::Result<Provider<Self>>
    where
        R: MessageCodecRegistry + 'static,
    {
        //TODO add remote config
        let Provider {
            name,
            provider,
            mut spawns,
        } = LocalActorRefProvider::new(name, config)?;
        register_remote_system_message(&mut registry);
        let remote_cfg = provider.config.get::<Remote>("akka.remote")?;
        let canonical = remote_cfg.artery.canonical;
        let transport = remote_cfg.artery.transport;
        let address = Address::new(Protocol::Tcp, name.clone(), Some(canonical));
        let (artery, artery_spawn) = RemoteActorRefProvider::spawn_artery(
            &provider,
            transport,
            canonical,
            remote_cfg.advanced.clone(),
        )?;
        spawns.push(artery_spawn);
        let (remote_watcher, remote_spawn) = Self::create_remote_watcher(&provider)?;
        spawns.push(remote_spawn);
        let remote = Self {
            remote_cfg,
            local: provider,
            address,
            artery,
            registry: Arc::new(registry),
            remote_watcher,
        };
        Ok(Provider::new(name, remote, spawns))
    }

    pub(crate) fn spawn_artery(
        provider: &LocalActorRefProvider,
        transport: Transport,
        socket_addr: SocketAddr,
        advanced: Advanced,
    ) -> anyhow::Result<(ActorRef, ActorSpawn)> {
        let props = Props::new_with_ctx::<_, ArteryActor>(move |context| {
            ArteryActor::new(context.system().clone(), transport, socket_addr, advanced)
        });
        let mailbox_cfg = LocalActorRef::get_mailbox_cfg(provider, &props)?;
        let actor_spawn = provider.system_guardian().attach_child_deferred(
            props,
            Some("artery".to_string()),
            None,
            mailbox_cfg,
        )?;
        let myself = actor_spawn.myself().clone();
        Ok((myself, actor_spawn))
    }

    fn has_address(&self, address: &Address) -> bool {
        address == self.local.root_path().address()
            || address == self.root_path().address()
            || address == &self.address
    }

    fn create_remote_watcher(
        provider: &LocalActorRefProvider,
    ) -> anyhow::Result<(ActorRef, ActorSpawn)> {
        let props = RemoteWatcher::props(Self::create_remote_watcher_failure_detector());
        let mailbox_cfg = LocalActorRef::get_mailbox_cfg(provider, &props)?;
        let actor_spawn = provider.system_guardian().attach_child_deferred(
            props,
            Some("remote_watcher".to_string()),
            None,
            mailbox_cfg,
        )?;
        let myself = actor_spawn.myself().clone();
        Ok((myself, actor_spawn))
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
}

impl TActorRefProvider for RemoteActorRefProvider {
    fn config(&self) -> &config::Config {
        self.local.config()
    }

    fn root_guardian(&self) -> &LocalActorRef {
        self.local.root_guardian()
    }

    fn root_guardian_at(&self, address: &Address) -> ActorRef {
        if self.has_address(address) {
            self.root_guardian().clone().into()
        } else {
            let remote = RemoteActorRef::new(
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

    fn spawn_actor(
        &self,
        props: Props,
        supervisor: &ActorRef,
        system: ActorSystem,
    ) -> anyhow::Result<ActorRef> {
        // TODO remote spawn
        self.local.spawn_actor(props, supervisor, system)
    }

    fn resolve_actor_ref_of_path(&self, path: &ActorPath) -> ActorRef {
        if path.address() == self.root_path().address() {
            self.local.resolve_actor_ref_of_path(path)
        } else {
            let remote = RemoteActorRef::new(
                path.clone(),
                self.artery.clone(),
                self.remote_watcher.clone(),
            );
            remote.into()
        }
    }

    fn dead_letters(&self) -> &dyn TActorRef {
        self.local.dead_letters()
    }

    fn ignore_ref(&self) -> &dyn TActorRef {
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

impl Into<ActorRefProvider> for RemoteActorRefProvider {
    fn into(self) -> ActorRefProvider {
        ActorRefProvider::new(self)
    }
}

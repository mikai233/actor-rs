use std::sync::Arc;

use anyhow::Context;
use tokio::sync::broadcast::Receiver;

use actor_core::actor::actor_path::ActorPath;
use actor_core::actor::actor_path::root_actor_path::RootActorPath;
use actor_core::actor::actor_path::TActorPath;
use actor_core::actor::actor_ref::{ActorRef, TActorRef};
use actor_core::actor::actor_ref_factory::ActorRefFactory;
use actor_core::actor::actor_ref_provider::{ActorRefProvider, TActorRefProvider};
use actor_core::actor::actor_system::ActorSystem;
use actor_core::actor::address::Address;
use actor_core::actor::local_actor_ref_provider::LocalActorRefProvider;
use actor_core::actor::local_ref::LocalActorRef;
use actor_core::actor::props::{ActorDeferredSpawn, DeferredSpawn, Props};
use actor_core::CodecMessage;
use actor_core::config::Config;
use actor_core::ext::option_ext::OptionExt;
use actor_core::message::message_registration::MessageRegistration;
use actor_derive::AsAny;

use crate::{REMOTE_CONFIG, REMOTE_CONFIG_NAME};
use crate::config::RemoteConfig;
use crate::config::transport::Transport;
use crate::net::tcp_transport::TcpTransportActor;
use crate::remote_actor_ref::RemoteActorRef;
use crate::remote_setting::RemoteSetting;

#[derive(Debug, AsAny)]
pub struct RemoteActorRefProvider {
    pub local: LocalActorRefProvider,
    pub address: Address,
    pub transport: ActorRef,
    pub registration: Arc<MessageRegistration>,
}

impl RemoteActorRefProvider {
    pub fn builder() -> RemoteProviderBuilder {
        RemoteProviderBuilder::new()
    }

    pub fn new(setting: RemoteSetting) -> anyhow::Result<(Self, Vec<Box<dyn DeferredSpawn>>)> {
        let RemoteSetting { system, config, reg } = setting;
        let default_config: RemoteConfig = toml::from_str(REMOTE_CONFIG).context(format!("failed to load {}", REMOTE_CONFIG_NAME))?;
        let remote_config = config.with_fallback(default_config);
        let transport = remote_config.transport.clone();
        let address = match &remote_config.transport {
            Transport::Tcp(tcp) => {
                Address {
                    protocol: tcp.name().to_string(),
                    system: system.name().clone(),
                    addr: Some(tcp.addr),
                }
            }
            Transport::Kcp(_) => {
                unimplemented!("kcp transport not unimplemented");
            }
        };
        system.add_config(remote_config)?;

        let (local, mut spawns) = LocalActorRefProvider::new(&system, Some(address.clone()))?;
        let (transport, deferred) = RemoteActorRefProvider::spawn_transport(&local, transport)?;
        deferred.into_foreach(|d| spawns.push(Box::new(d)));
        let remote = Self {
            local,
            address,
            transport,
            registration: Arc::new(reg),
        };
        Ok((remote, spawns))
    }

    pub(crate) fn spawn_transport(provider: &LocalActorRefProvider, transport: Transport) -> anyhow::Result<(ActorRef, Option<ActorDeferredSpawn>)> {
        match transport {
            Transport::Tcp(tcp) => {
                provider.system_guardian().attach_child(
                    Props::create(move |context| Ok(TcpTransportActor::new(context.system().clone(), tcp.clone()))),
                    Some("tcp_transport".to_string()),
                    false,
                )
            }
            Transport::Kcp(_) => {
                unimplemented!("kcp transport not unimplemented");
            }
        }
    }

    fn has_address(&self, address: &Address) -> bool {
        address == self.local.root_path().address() || address == self.root_path().address() || address == &self.address
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
                self.transport.clone(),
                self.registration.clone(),
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

    fn temp_path_of_prefix(&self, prefix: Option<&String>) -> ActorPath {
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

    fn spawn_actor(&self, props: Props, supervisor: &ActorRef) -> anyhow::Result<ActorRef> {
        // TODO remote spawn
        self.local.spawn_actor(props, supervisor)
    }

    fn resolve_actor_ref_of_path(&self, path: &ActorPath) -> ActorRef {
        if path.address() == self.root_path().address() {
            self.local.resolve_actor_ref_of_path(path)
        } else {
            let remote = RemoteActorRef::new(
                self.transport.system().clone(),
                path.clone(),
                self.transport.clone(),
                self.registration.clone(),
            );
            remote.into()
        }
    }

    fn dead_letters(&self) -> &ActorRef {
        &self.local.dead_letters()
    }

    fn registration(&self) -> Option<&Arc<MessageRegistration>> {
        Some(&self.registration)
    }

    fn termination_rx(&self) -> Receiver<()> {
        self.local.termination_rx()
    }
}

impl Into<ActorRefProvider> for RemoteActorRefProvider {
    fn into(self) -> ActorRefProvider {
        ActorRefProvider::new(self)
    }
}

pub struct RemoteProviderBuilder {
    reg: MessageRegistration,
    config: Option<RemoteConfig>,
}

impl RemoteProviderBuilder {
    pub fn new() -> Self {
        Self {
            reg: MessageRegistration::new(),
            config: None,
        }
    }

    pub fn config(mut self, config: RemoteConfig) -> Self {
        self.config = Some(config);
        self
    }

    pub fn register<M>(mut self) -> Self where M: CodecMessage {
        self.reg.register_user::<M>();
        self
    }

    pub fn build(self, system: ActorSystem) -> anyhow::Result<(ActorRefProvider, Vec<Box<dyn DeferredSpawn>>)> {
        let Self { reg, config } = self;
        let setting = RemoteSetting::builder()
            .system(system)
            .config(config.into_result()?)
            .reg(reg)
            .build();
        RemoteActorRefProvider::new(setting).map(|(c, d)| (c.into(), d))
    }
}
use std::net::SocketAddrV4;
use std::sync::Arc;

use actor_core::actor::actor_path::ActorPath;
use actor_core::actor::actor_path::TActorPath;
use actor_core::actor::actor_ref::ActorRef;
use actor_core::actor::actor_ref_factory::ActorRefFactory;
use actor_core::actor::actor_ref_provider::{ActorRefProvider, TActorRefProvider};
use actor_core::actor::actor_system::ActorSystem;
use actor_core::actor::address::Address;
use actor_core::actor::local_actor_ref_provider::LocalActorRefProvider;
use actor_core::actor::local_ref::LocalActorRef;
use actor_core::actor::props::{DeferredSpawn, Props};
use actor_core::ext::option_ext::OptionExt;

use crate::message_registration::MessageRegistration;
use crate::net::tcp_transport::TransportActor;
use crate::remote_actor_ref::{Inner, RemoteActorRef};

#[derive(Debug)]
pub struct RemoteActorRefProvider {
    pub local: LocalActorRefProvider,
    pub address: Address,
    pub transport: ActorRef,
    pub registration: Arc<MessageRegistration>,
}

impl RemoteActorRefProvider {
    pub fn new(system: &ActorSystem, registration: MessageRegistration, addr: SocketAddrV4) -> anyhow::Result<(Self, Vec<DeferredSpawn>)> {
        let address = Address {
            protocol: "tcp".to_string(),
            system: system.name().clone(),
            addr: Some(addr),
        };
        let (local, mut deferred_vec) = LocalActorRefProvider::new(&system, Some(address.clone()))?;
        let (transport, deferred) = RemoteActorRefProvider::spawn_transport(&local)?;
        deferred.into_foreach(|d| deferred_vec.push(d));
        let remote = Self {
            local,
            address,
            transport,
            registration: Arc::new(registration),
        };
        Ok((remote, deferred_vec))
    }
    pub(crate) fn spawn_transport(provider: &LocalActorRefProvider) -> anyhow::Result<(ActorRef, Option<DeferredSpawn>)> {
        provider.system_guardian().attach_child(
            Props::create(|context| TransportActor::new(context.system().clone())),
            Some("tcp_transport".to_string()),
            false,
        )
    }
}

impl TActorRefProvider for RemoteActorRefProvider {
    fn root_guardian(&self) -> &LocalActorRef {
        self.local.root_guardian()
    }

    fn root_guardian_at(&self, address: &Address) -> ActorRef {
        todo!()
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

    fn temp_path_of_prefix(&self, prefix: Option<String>) -> ActorPath {
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

    fn actor_of(&self, props: Props, supervisor: &ActorRef) -> anyhow::Result<ActorRef> {
        // TODO remote spawn
        self.local.actor_of(props, supervisor)
    }

    fn resolve_actor_ref_of_path(&self, path: &ActorPath) -> ActorRef {
        if path.address() == self.root_path().address() {
            self.local.resolve_actor_ref_of_path(path)
        } else {
            let inner = Inner {
                system: self.transport.system(),
                path: path.clone(),
                transport: Arc::new(self.transport.clone()),
                registration: self.registration.clone(),
            };
            let remote = RemoteActorRef {
                inner: inner.into()
            };
            remote.into()
        }
    }

    fn dead_letters(&self) -> &ActorRef {
        &self.local.dead_letters()
    }
}

impl Into<ActorRefProvider> for RemoteActorRefProvider {
    fn into(self) -> ActorRefProvider {
        ActorRefProvider::new(self)
    }
}
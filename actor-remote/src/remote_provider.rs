use std::sync::Arc;

use actor_core::actor_path::ActorPath;
use actor_core::actor_path::TActorPath;
use actor_core::actor_ref::ActorRef;
use actor_core::actor_ref::local_ref::LocalActorRef;
use actor_core::actor_ref::remote_ref::{Inner, RemoteActorRef};
use actor_core::actor::actor_ref_provider::ActorRefProvider;
use actor_core::local_actor_ref_provider::LocalActorRefProvider;
use actor_core::props::Props;
use actor_core::provider::{ActorRefFactory, ActorRefProvider};
use actor_core::provider::local_provider::LocalActorRefProvider;
use actor_core::system::ActorSystem;

#[derive(Debug)]
pub struct RemoteActorRefProvider {
    pub local: LocalActorRefProvider,
    pub transport: ActorRef,
}

impl RemoteActorRefProvider {
    pub fn init(system: &ActorSystem) -> anyhow::Result<()> {
        let local = LocalActorRefProvider::new(&system)?;
        let transport = local.start_tcp_transport()?;
        let provider = Box::new(Self { local, transport });
        system.provider.store(Arc::new(provider));
        Ok(())
    }
    pub(crate) fn start_tcp_transport(&self) -> anyhow::Result<ActorRef> {
        let transport_ref = self
            .system_guardian()
            .attach_child(Props::create(|context| TransportActor::new(context.provider())), Some("tcp_transport".to_string()))?;
        Ok(transport_ref)
    }
}

impl ActorRefProvider for RemoteActorRefProvider {
    fn root_guardian(&self) -> &LocalActorRef {
        self.local.root_guardian()
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
            let system = self.system_guardian().system.clone();
            let provider = system.provider();
            let remote = provider.remote_or_panic();
            let inner = Inner {
                system: self.system_guardian().system.clone(),
                path: path.clone(),
                transport: Arc::new(remote.transport.clone()),
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

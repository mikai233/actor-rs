use std::sync::Arc;

use crate::actor::Actor;
use crate::actor_path::ActorPath;
use crate::actor_path::TActorPath;
use crate::actor_ref::ActorRef;
use crate::actor_ref::local_ref::LocalActorRef;
use crate::actor_ref::remote_ref::RemoteActorRef;
use crate::props::Props;
use crate::provider::{ActorRefFactory, TActorRefProvider};
use crate::provider::local_provider::LocalActorRefProvider;
use crate::system::ActorSystem;

#[derive(Debug, Clone)]
pub struct RemoteActorRefProvider {
    pub(crate) local: LocalActorRefProvider,
    pub(crate) transport: ActorRef,
}

impl RemoteActorRefProvider {
    pub(crate) fn init(system: &ActorSystem) -> anyhow::Result<()> {
        let local = LocalActorRefProvider::new(&system)?;
        let transport = local.spawn_tcp_transport(system)?;
        let provider = Self { local, transport };
        *system.provider_rw().write().unwrap() = provider.into();
        Ok(())
    }
}

impl TActorRefProvider for RemoteActorRefProvider {
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

    fn register_temp_actor(&self, actor: ActorRef, path: ActorPath) {
        todo!()
    }

    fn unregister_temp_actor(&self, path: ActorPath) {
        todo!()
    }

    fn actor_of<T>(
        &self,
        actor: T,
        arg: T::A,
        props: Props,
        supervisor: &ActorRef,
    ) -> anyhow::Result<ActorRef>
        where
            T: Actor,
    {
        //TODO remote spawn
        self.local.actor_of(actor, arg, props, supervisor)
    }

    fn resolve_actor_ref_of_path(&self, path: &ActorPath) -> ActorRef {
        if path.address() == self.root_path().address() {
            self.local.resolve_actor_ref_of_path(path)
        } else {
            let system = self.system_guardian().system.clone();
            let provider = system.provider();
            let remote = provider.remote_or_panic();
            let remote = RemoteActorRef {
                system: self.system_guardian().system.clone(),
                path: path.clone(),
                transport: Arc::new(remote.transport.clone()),
            };
            remote.into()
        }
    }

    fn dead_letters(&self) -> &ActorRef {
        &self.local.dead_letters()
    }
}
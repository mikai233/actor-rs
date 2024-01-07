use actor_core::actor::actor_path::ActorPath;
use actor_core::actor::actor_ref::ActorRef;
use actor_core::actor::actor_ref_provider::TActorRefProvider;
use actor_core::actor::actor_system::ActorSystem;
use actor_core::actor::address::Address;
use actor_core::actor::local_ref::LocalActorRef;
use actor_core::actor::props::Props;
use actor_derive::AsAny;
use actor_remote::remote_provider::RemoteActorRefProvider;

#[derive(Debug, Clone, AsAny)]
pub struct ClusterActorRefProvider {
    pub(crate) remote: RemoteActorRefProvider,
}

impl ClusterActorRefProvider {
    pub fn new(system: ActorSystem) -> Self {
        todo!()
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
}
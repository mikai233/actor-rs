use crate::actor::Actor;
use crate::actor_path::ActorPath;
use crate::actor_ref::ActorRef;
use crate::actor_ref::local_ref::LocalActorRef;
use crate::props::Props;
use crate::provider::local_provider::LocalActorRefProvider;
use crate::provider::TActorRefProvider;

#[derive(Debug, Clone)]
pub struct RemoteActorRefProvider {
    pub(crate) local: LocalActorRefProvider,
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

    fn temp_path(&self) -> &ActorPath {
        todo!()
    }

    fn temp_path_of(&self) -> &ActorPath {
        todo!()
    }

    fn register_temp_actor(&self, actor: ActorRef, path: ActorPath) {
        todo!()
    }

    fn unregister_temp_actor(&self, path: ActorPath) {
        todo!()
    }

    fn actor_of<T>(&self, actor: T, arg: T::A, props: Props, supervisor: &ActorRef, path: ActorPath) -> anyhow::Result<ActorRef> where T: Actor {
        //TODO remote spawn
        self.local.actor_of(actor, arg, props, supervisor, path)
    }

    fn resolve_actor_ref(&self, path: String) -> ActorRef {
        todo!()
    }
    fn resolve_actor_ref_of_path(&self, path: ActorPath) -> ActorRef {
        todo!()
    }
}
use std::sync::Arc;

use crate::actor::actor_path::ActorPath;
use crate::actor::actor_ref::ActorRef;
use crate::actor::actor_ref_provider::ActorRefProvider;
use crate::actor_ref::local_ref::LocalActorRef;
use crate::props::Props;
use crate::system::ActorSystem;

pub trait ActorRefFactory {
    fn system(&self) -> &ActorSystem;
    fn provider(&self) -> Arc<Box<dyn ActorRefProvider>>;
    fn guardian(&self) -> LocalActorRef;
    fn lookup_root(&self) -> ActorRef;
    fn spawn_actor(&self, props: Props, name: impl Into<String>) -> anyhow::Result<ActorRef>;
    fn spawn_anonymous_actor(&self, props: Props) -> anyhow::Result<ActorRef>;
    fn actor_selection(&self, _path: String) {
        todo!()
    }
    fn actor_selection_of_path(&self, _path: ActorPath) {
        todo!()
    }
    fn stop(&self, actor: &ActorRef);
}
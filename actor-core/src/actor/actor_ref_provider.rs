use std::any::Any;
use crate::actor::actor_path::ActorPath;

use crate::actor::actor_ref::ActorRef;
use crate::actor_ref::local_ref::LocalActorRef;
use crate::ext::as_any::AsAny;
use crate::props::Props;

pub trait ActorRefProvider: Send + Any + AsAny {
    fn root_guardian(&self) -> &LocalActorRef;
    fn guardian(&self) -> &LocalActorRef;
    fn system_guardian(&self) -> &LocalActorRef;
    fn root_path(&self) -> &ActorPath;
    fn temp_path(&self) -> ActorPath;
    fn temp_path_of_prefix(&self, prefix: Option<String>) -> ActorPath;
    fn temp_container(&self) -> ActorRef;
    fn register_temp_actor(&self, actor: ActorRef, path: &ActorPath);
    fn unregister_temp_actor(&self, path: &ActorPath);
    fn actor_of(&self, props: Props, supervisor: &ActorRef) -> anyhow::Result<ActorRef>;
    fn resolve_actor_ref(&self, path: impl AsRef<str>) -> ActorRef {
        match path.as_ref().parse::<ActorPath>() {
            Ok(actor_path) => self.resolve_actor_ref_of_path(&actor_path),
            Err(_) => self.dead_letters().clone(),
        }
    }
    fn resolve_actor_ref_of_path(&self, path: &ActorPath) -> ActorRef;
    fn dead_letters(&self) -> &ActorRef;
}
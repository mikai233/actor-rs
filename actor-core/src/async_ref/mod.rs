use std::any::Any;
use std::fmt::Debug;
use std::iter::Peekable;

use async_trait::async_trait;

use crate::actor::actor_system::WeakActorSystem;
use crate::actor_path::ActorPath;
use crate::actor_ref::ActorRef;
use crate::DynMessage;
use crate::ext::as_any::AsAny;

#[async_trait]
pub trait TAsyncActorRef: Debug + Send + Sync + Any + AsAny {
    fn system(&self) -> &WeakActorSystem;

    fn path(&self) -> &ActorPath;

    async fn tell(&self, message: DynMessage, sender: Option<ActorRef>);

    fn stop(&self);

    fn parent(&self) -> Option<&ActorRef>;

    fn get_child(&self, names: &mut Peekable<&mut dyn Iterator<Item=&str>>) -> Option<ActorRef>;

    fn resume(&self) {}

    fn suspend(&self) {}
}
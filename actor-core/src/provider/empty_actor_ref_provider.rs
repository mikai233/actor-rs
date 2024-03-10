use std::fmt::Debug;
use std::sync::Arc;

use tokio::sync::broadcast::Receiver;

use actor_derive::AsAny;

use crate::actor::address::Address;
use crate::actor::props::Props;
use crate::actor_path::ActorPath;
use crate::actor_ref::ActorRef;
use crate::actor_ref::local_ref::LocalActorRef;
use crate::message::message_registration::MessageRegistration;
use crate::provider::{ActorRefProvider, TActorRefProvider};

#[derive(Debug, Default, Clone, Copy, AsAny)]
pub(crate) struct EmptyActorRefProvider;

impl TActorRefProvider for EmptyActorRefProvider {
    fn root_guardian(&self) -> &LocalActorRef {
        unimplemented!()
    }

    fn root_guardian_at(&self, _address: &Address) -> ActorRef {
        unimplemented!()
    }

    fn guardian(&self) -> &LocalActorRef {
        unimplemented!()
    }

    fn system_guardian(&self) -> &LocalActorRef {
        unimplemented!()
    }

    fn root_path(&self) -> &ActorPath {
        unimplemented!()
    }

    fn temp_path(&self) -> ActorPath {
        unimplemented!()
    }

    fn temp_path_of_prefix(&self, _prefix: Option<&String>) -> ActorPath {
        unimplemented!()
    }

    fn temp_container(&self) -> ActorRef {
        unimplemented!()
    }

    fn register_temp_actor(&self, _actor: ActorRef, _path: &ActorPath) {
        unimplemented!()
    }

    fn unregister_temp_actor(&self, _path: &ActorPath) {
        unimplemented!()
    }

    fn spawn_actor(&self, _props: Props, _supervisor: &ActorRef) -> anyhow::Result<ActorRef> {
        unimplemented!()
    }

    fn resolve_actor_ref_of_path(&self, _path: &ActorPath) -> ActorRef {
        unimplemented!()
    }

    fn dead_letters(&self) -> &ActorRef {
        unimplemented!()
    }

    fn registration(&self) -> Option<&Arc<MessageRegistration>> {
        None
    }

    fn termination_rx(&self) -> Receiver<()> {
        unimplemented!()
    }
}

impl Into<ActorRefProvider> for EmptyActorRefProvider {
    fn into(self) -> ActorRefProvider {
        ActorRefProvider::new(self)
    }
}
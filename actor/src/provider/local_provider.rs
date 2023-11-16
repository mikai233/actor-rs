use std::sync::Arc;

use crate::actor::Actor;
use crate::actor_path::{ActorPath, RootActorPath, TActorPath};
use crate::actor_ref::{ActorRef, TActorRef};
use crate::actor_ref::dead_letter_ref::DeadLetterActorRef;
use crate::actor_ref::local_ref::LocalActorRef;
use crate::actor_ref::virtual_path_container::VirtualPathContainer;
use crate::cell::ActorCell;
use crate::ext::random_actor_name;
use crate::net::tcp_transport::TransportActor;
use crate::props::Props;
use crate::provider::TActorRefProvider;
use crate::root_guardian::RootGuardian;
use crate::system::ActorSystem;
use crate::system_guardian::SystemGuardian;
use crate::user_guardian::UserGuardian;

#[derive(Debug, Clone)]
pub struct LocalActorRefProvider {
    inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    root_path: ActorPath,
    root_guardian: LocalActorRef,
    user_guardian: LocalActorRef,
    system_guardian: LocalActorRef,
    dead_letters: ActorRef,
    temp_node: ActorPath,
    temp_container: VirtualPathContainer,
}

impl LocalActorRefProvider {
    pub(crate) fn new(system: &ActorSystem) -> anyhow::Result<Self> {
        let root_path = RootActorPath::new(system.address().clone(), "/".to_string());
        let root_props = Props::default();
        let (sender, mailbox) = root_props.mailbox();
        let root_guardian = LocalActorRef {
            system: system.clone(),
            path: root_path.clone().into(),
            sender,
            cell: ActorCell::new(None),
        };
        root_guardian.start(RootGuardian, (), root_props, mailbox);
        let system_guardian = root_guardian.attach_child(SystemGuardian, (), Some("system".to_string()), Props::default())?;
        let user_guardian = root_guardian.attach_child(UserGuardian, (), Some("user".to_string()), Props::default())?;
        let dead_letters = DeadLetterActorRef {
            system: system.clone(),
            path: root_path.child("deadLetters"),
        };
        let temp_node = root_path.child("temp");
        let temp_container = VirtualPathContainer {
            system: system.clone(),
            path: temp_node.clone(),
            parent: Box::new(root_guardian.clone().into()),
            children: Arc::new(Default::default()),
        };
        let inner = Inner {
            root_path: root_path.into(),
            root_guardian,
            user_guardian: user_guardian.local_or_panic().clone().into(),
            system_guardian: system_guardian.local_or_panic().clone().into(),
            dead_letters: dead_letters.into(),
            temp_node,
            temp_container,
        };
        let provider = Self {
            inner: inner.into(),
        };
        Ok(provider)
    }

    pub(crate) fn spawn_tcp_transport(&self, system: &ActorSystem) -> anyhow::Result<ActorRef> {
        let transport_ref = self
            .system_guardian()
            .attach_child(
                TransportActor,
                (),
                Some("tcp_transport".to_string()),
                Props::default())?;
        Ok(transport_ref)
    }
}

impl TActorRefProvider for LocalActorRefProvider {
    fn root_guardian(&self) -> &LocalActorRef {
        &self.inner.root_guardian
    }

    fn guardian(&self) -> &LocalActorRef {
        &self.inner.user_guardian
    }

    fn system_guardian(&self) -> &LocalActorRef {
        &self.inner.system_guardian
    }

    fn root_path(&self) -> &ActorPath {
        &self.inner.root_path
    }

    fn temp_path(&self) -> ActorPath {
        self.temp_path_of_prefix(None)
    }

    fn temp_path_of_prefix(&self, prefix: Option<String>) -> ActorPath {
        let mut builder = String::new();
        let prefix_is_none_or_empty = prefix.as_ref().map(|p| p.is_empty()).unwrap_or(true);
        if !prefix_is_none_or_empty {
            builder.push_str(prefix.unwrap().as_str());
        }
        builder.push_str(random_actor_name().as_str());
        self.inner.temp_node.child(&builder)
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
        supervisor.local_or_panic().attach_child(actor, arg, None, props)
    }

    fn resolve_actor_ref_of_path(&self, path: &ActorPath) -> ActorRef {
        if path.address() == self.root_path().address() {
            self.root_guardian()
                .get_child(path.elements())
                .unwrap_or_else(|| self.dead_letters().clone())
        } else {
            self.dead_letters().clone()
        }
    }

    fn dead_letters(&self) -> &ActorRef {
        &self.inner.dead_letters
    }
}

#[cfg(test)]
mod local_provider_test {
    use tracing::info;

    use crate::actor::Actor;
    use crate::actor::context::ActorContext;
    use crate::actor_ref::ActorRefExt;
    use crate::message::MessageRegistration;
    use crate::props::Props;
    use crate::provider::{ActorRefFactory, TActorRefProvider};
    use crate::system::ActorSystem;

    #[derive(Debug)]
    struct ActorA;

    impl Actor for ActorA {
        type S = ();
        type A = ();

        fn pre_start(&self, context: &mut ActorContext, arg: Self::A) -> anyhow::Result<Self::S> {
            info!("actor a {} pre start", context.myself);
            context.actor_of(ActorB, (), Props::default(), None)?;
            Ok(())
        }
    }

    #[derive(Debug)]
    struct ActorB;

    impl Actor for ActorB {
        type S = ();
        type A = ();

        fn pre_start(&self, context: &mut ActorContext, arg: Self::A) -> anyhow::Result<Self::S> {
            info!("actor b {} pre start", context.myself);
            context.actor_of(ActorC, (), Props::default(), None)?;
            Ok(())
        }
    }

    #[derive(Debug)]
    struct ActorC;

    impl Actor for ActorC {
        type S = ();
        type A = ();

        fn pre_start(&self, context: &mut ActorContext, arg: Self::A) -> anyhow::Result<Self::S> {
            info!("actor c {} pre start", context.myself);
            Ok(())
        }
    }

    #[test]
    fn test() -> anyhow::Result<()> {
        let system = ActorSystem::new("game".to_string(), "127.0.0.1:12121".parse()?, MessageRegistration::new())?;
        let actor_a = system.actor_of(ActorA, (), Props::default(), None)?;
        let actor_c = system
            .provider()
            .resolve_actor_ref(&"tcp://game@127.0.0.1:12121/user/$a/$b/$c".to_string());
        actor_c.cast((), None);
        std::thread::park();
        Ok(())
    }
}

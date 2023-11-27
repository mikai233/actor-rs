use std::sync::Arc;

use crate::Actor;
use crate::actor_path::{ActorPath, RootActorPath, TActorPath};
use crate::actor_ref::{ActorRef, TActorRef};
use crate::actor_ref::dead_letter_ref::DeadLetterActorRef;
use crate::actor_ref::local_ref::LocalActorRef;
use crate::actor_ref::virtual_path_container::VirtualPathContainer;
use crate::cell::ActorCell;
use crate::ext::random_actor_name;
use crate::net::tcp_transport::TransportActor;
use crate::props::{noarg_props, Props};
use crate::provider::TActorRefProvider;
use crate::system::ActorSystem;
use crate::system::root_guardian::RootGuardian;
use crate::system::system_guardian::SystemGuardian;
use crate::system::user_guardian::UserGuardian;

#[derive(Debug)]
pub struct LocalActorRefProvider {
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
        let root_props = noarg_props::<RootGuardian>();
        let (sender, mailbox) = root_props.mailbox();
        let inner = crate::actor_ref::local_ref::Inner {
            system: system.clone(),
            path: root_path.clone().into(),
            sender,
            cell: ActorCell::new(None),
        };
        let root_guardian = LocalActorRef {
            inner: inner.into(),
        };
        root_guardian.start::<RootGuardian>((), mailbox);
        let system_guardian = root_guardian.attach_child(noarg_props::<SystemGuardian>(), Some("system".to_string()))?;
        let system_guardian = system_guardian.local_or_panic();
        let user_guardian = root_guardian.attach_child(noarg_props::<UserGuardian>(), Some("user".to_string()))?;
        let user_guardian = user_guardian.local_or_panic();
        let inner = crate::actor_ref::dead_letter_ref::Inner {
            system: system.clone(),
            path: root_path.child("deadLetters"),
        };
        let dead_letters = DeadLetterActorRef {
            inner: inner.into(),
        };
        // root_guardian.cell.children().write().unwrap().insert(dead_letters.path.name().clone(), dead_letters.clone().into());
        let temp_node = root_path.child("temp");
        let inner = crate::actor_ref::virtual_path_container::Inner {
            system: system.clone(),
            path: temp_node.clone(),
            parent: Box::new(root_guardian.clone().into()),
            children: Arc::new(Default::default()),
        };
        let temp_container = VirtualPathContainer {
            inner: inner.into(),
        };
        root_guardian.cell.children().write().unwrap().insert(temp_node.name().clone(), temp_container.clone().into());
        let provider = LocalActorRefProvider {
            root_path: root_path.into(),
            root_guardian,
            user_guardian: user_guardian.clone().into(),
            system_guardian: system_guardian.clone().into(),
            dead_letters: dead_letters.into(),
            temp_node,
            temp_container,
        };
        Ok(provider)
    }

    pub(crate) fn start_tcp_transport(&self) -> anyhow::Result<ActorRef> {
        let transport_ref = self
            .system_guardian()
            .attach_child(noarg_props::<TransportActor>(), Some("tcp_transport".to_string()))?;
        Ok(transport_ref)
    }
}

impl TActorRefProvider for LocalActorRefProvider {
    fn root_guardian(&self) -> &LocalActorRef {
        &self.root_guardian
    }

    fn guardian(&self) -> &LocalActorRef {
        &self.user_guardian
    }

    fn system_guardian(&self) -> &LocalActorRef {
        &self.system_guardian
    }

    fn root_path(&self) -> &ActorPath {
        &self.root_path
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
        self.temp_node.child(&builder)
    }

    fn temp_container(&self) -> ActorRef {
        self.temp_container.clone().into()
    }

    fn register_temp_actor(&self, actor: ActorRef, path: &ActorPath) {
        assert_eq!(path.parent(), self.temp_node, "cannot register_temp_actor() with anything not obtained from temp_path()");
        self.temp_container.add_child(path.name().to_string(), actor);
    }

    fn unregister_temp_actor(&self, path: &ActorPath) {
        assert_eq!(path.parent(), self.temp_node, "cannot unregister_temp_actor() with anything not obtained from temp_path()");
        self.temp_container.remove_child(path.name());
    }

    fn actor_of<T>(&self, props: Props<T>, supervisor: &ActorRef) -> anyhow::Result<ActorRef>
        where
            T: Actor,
    {
        supervisor.local_or_panic().attach_child(props, None)
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
        &self.dead_letters
    }
}

#[cfg(test)]
mod local_provider_test {
    use async_trait::async_trait;
    use tracing::info;

    use crate::{Actor, EmptyTestActor, EmptyTestMessage};
    use crate::actor_ref::ActorRefExt;
    use crate::context::ActorContext;
    use crate::props::noarg_props;
    use crate::provider::{ActorRefFactory, TActorRefProvider};
    use crate::system::ActorSystem;
    use crate::system::config::Config;

    #[derive(Debug)]
    struct ActorA;

    #[async_trait]
    impl Actor for ActorA {
        type S = ();
        type A = ();

        async fn pre_start(context: &mut ActorContext, _arg: Self::A) -> anyhow::Result<Self::S> {
            info!("actor a {} pre start", context.myself);
            context.actor_of(noarg_props::<ActorA>(), None)?;
            Ok(())
        }
    }

    #[derive(Debug)]
    struct ActorB;

    #[async_trait]
    impl Actor for ActorB {
        type S = ();
        type A = ();

        async fn pre_start(context: &mut ActorContext, _arg: Self::A) -> anyhow::Result<Self::S> {
            info!("actor b {} pre start", context.myself);
            context.actor_of(noarg_props::<EmptyTestActor>(), None)?;
            Ok(())
        }
    }


    #[tokio::test]
    async fn test() -> anyhow::Result<()> {
        let system = ActorSystem::create(Config::default()).await?;
        let _ = system.actor_of(noarg_props::<ActorA>(), None)?;
        let actor_c = system
            .provider()
            .resolve_actor_ref(&"tcp://game@127.0.0.1:12121/user/$a/$b/$c".to_string());
        actor_c.cast(EmptyTestMessage, None);
        std::thread::park();
        Ok(())
    }
}

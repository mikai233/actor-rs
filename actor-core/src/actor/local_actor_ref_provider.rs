use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use dashmap::DashMap;

use crate::actor::actor_path::{ActorPath, RootActorPath, TActorPath};
use crate::actor::actor_ref::ActorRef;
use crate::actor::actor_ref::TActorRef;
use crate::actor::actor_ref_provider::ActorRefProvider;
use crate::actor_ref::dead_letter_ref::DeadLetterActorRef;
use crate::actor_ref::local_ref::LocalActorRef;
use crate::actor_ref::virtual_path_container::VirtualPathContainer;
use crate::cell::ActorCell;
use crate::ext::base64;
use crate::props::Props;
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
    extra_names: DashMap<String, ActorRef>,
    dead_letters: ActorRef,
    temp_number: AtomicI64,
    temp_node: ActorPath,
    temp_container: VirtualPathContainer,
}

impl LocalActorRefProvider {
    pub fn new(system: &ActorSystem) -> anyhow::Result<Self> {
        let root_path = RootActorPath::new(system.address().clone(), "/".to_string());
        let root_props = Props::create(|_| { RootGuardian::default() });
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
        (root_props.spawner)(root_guardian.clone().into(), mailbox, system.clone());
        let system_guardian = root_guardian.attach_child(Props::create(|_| SystemGuardian), Some("system".to_string()))?;
        let system_guardian = system_guardian.local_or_panic();
        let user_guardian = root_guardian.attach_child(Props::create(|_| UserGuardian), Some("user".to_string()))?;
        let user_guardian = user_guardian.local_or_panic();
        let inner = crate::actor_ref::dead_letter_ref::Inner {
            system: system.clone(),
            path: root_path.child("dead_letters"),
        };
        let dead_letters = DeadLetterActorRef { inner: inner.into() };
        let temp_node = root_path.child("temp");
        let inner = crate::actor_ref::virtual_path_container::Inner {
            system: system.clone(),
            path: temp_node.clone(),
            parent: root_guardian.clone().into(),
            children: Arc::new(Default::default()),
        };
        let temp_container = VirtualPathContainer {
            inner: inner.into(),
        };
        let provider = LocalActorRefProvider {
            root_path: root_path.into(),
            root_guardian,
            user_guardian: user_guardian.clone().into(),
            system_guardian: system_guardian.clone().into(),
            extra_names: DashMap::new(),
            dead_letters: dead_letters.into(),
            temp_number: AtomicI64::new(0),
            temp_node,
            temp_container,
        };
        Ok(provider)
    }
}

impl ActorRefProvider for LocalActorRefProvider {
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
        builder.push_str("$");
        let builder = base64(self.temp_number.fetch_add(1, Ordering::Relaxed), builder);
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

    fn actor_of(&self, props: Props, supervisor: &ActorRef) -> anyhow::Result<ActorRef> {
        supervisor.local_or_panic().attach_child(props, None)
    }

    fn resolve_actor_ref_of_path(&self, path: &ActorPath) -> ActorRef {
        if path.address() == self.root_path().address() {
            let elements = path.elements();
            match elements.iter().peekable().peek() {
                Some(peek) if peek.as_str() == "temp" => {
                    let mut iter = elements.into_iter();
                    iter.next();
                    TActorRef::get_child(&self.temp_container, iter).unwrap_or_else(|| self.dead_letters.clone())
                }
                Some(peek) if peek.as_str() == "dead_letters" => {
                    self.dead_letters.clone()
                }
                Some(peek) if self.extra_names.contains_key(*peek) => {
                    self.extra_names.get(*peek).map(|r| r.value().clone()).unwrap_or_else(|| self.dead_letters.clone())
                }
                _ => {
                    self.root_guardian()
                        .get_child(elements)
                        .unwrap_or_else(|| self.dead_letters().clone())
                }
            }
        } else {
            self.dead_letters().clone()
        }
    }

    fn dead_letters(&self) -> &ActorRef {
        &self.dead_letters
    }
}

// #[cfg(test)]
// mod local_provider_test {
//     use async_trait::async_trait;
//     use tracing::info;
//
//     use crate::{Actor, EmptyTestActor, EmptyTestMessage};
//     use crate::actor::actor_ref::ActorRefExt;
//     use crate::actor_ref_factory::ActorRefFactory;
//     use crate::actor::context::ActorContext;
//     use crate::props::Props;
//     use crate::system::ActorSystem;
//     use crate::system::config::ActorSystemConfig;
//
//     #[derive(Debug)]
//     struct ActorA;
//
//     #[async_trait]
//     impl Actor for ActorA {
//         async fn pre_start(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
//             info!("actor a {} pre start", context.myself);
//             context.spawn_anonymous_actor(Props::create(|_| ActorA))?;
//             Ok(())
//         }
//     }
//
//     #[derive(Debug)]
//     struct ActorB;
//
//     #[async_trait]
//     impl Actor for ActorB {
//         async fn pre_start(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
//             info!("actor b {} pre start", context.myself);
//             context.spawn_anonymous_actor(Props::create(|_| EmptyTestActor))?;
//             Ok(())
//         }
//     }
//
//
//     #[tokio::test]
//     async fn test() -> anyhow::Result<()> {
//         let system = ActorSystem::create(ActorSystemConfig::default()).await?;
//         let _ = system.spawn_anonymous_actor(Props::create(|_| ActorA))?;
//         let actor_c = system
//             .provider()
//             .resolve_actor_ref(&"tcp://game@127.0.0.1:12121/user/$a/$b/$c".to_string());
//         actor_c.cast(EmptyTestMessage, None);
//         std::thread::park();
//         Ok(())
//     }
// }

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use dashmap::DashMap;

use actor_derive::AsAny;

use crate::actor::actor_path::{ActorPath, TActorPath};
use crate::actor::actor_path::root_actor_path::RootActorPath;
use crate::actor::actor_ref::ActorRef;
use crate::actor::actor_ref::TActorRef;
use crate::actor::actor_ref_provider::{ActorRefProvider, TActorRefProvider};
use crate::actor::actor_system::ActorSystem;
use crate::actor::address::Address;
use crate::actor::dead_letter_ref::DeadLetterActorRef;
use crate::actor::local_ref::LocalActorRef;
use crate::actor::props::{DeferredSpawn, Props};
use crate::actor::root_guardian::RootGuardian;
use crate::actor::system_guardian::SystemGuardian;
use crate::actor::user_guardian::UserGuardian;
use crate::actor::virtual_path_container::VirtualPathContainer;
use crate::cell::ActorCell;
use crate::ext::base64;
use crate::ext::option_ext::OptionExt;

#[derive(Debug, AsAny)]
pub struct LocalActorRefProvider {
    root_path: ActorPath,
    root_guardian: LocalActorRef,
    user_guardian: LocalActorRef,
    system_guardian: LocalActorRef,
    extra_names: DashMap<String, ActorRef, ahash::RandomState>,
    dead_letters: ActorRef,
    temp_number: AtomicI64,
    temp_node: ActorPath,
    temp_container: VirtualPathContainer,
}

impl LocalActorRefProvider {
    pub fn new(system: &ActorSystem, address: Option<Address>) -> anyhow::Result<(Self, Vec<DeferredSpawn>)> {
        let mut deferred_spawns = vec![];
        let address = address.unwrap_or_else(|| {
            Address {
                protocol: "tcp".to_string(),
                system: system.name().clone(),
                addr: None,
            }
        });
        let root_path = RootActorPath::new(address, "/");
        let root_props = Props::create(|_| { RootGuardian::default() });
        let (sender, mailbox) = root_props.mailbox();
        let inner = crate::actor::local_ref::Inner {
            system: system.clone(),
            path: root_path.clone().into(),
            sender,
            cell: ActorCell::new(None),
        };
        let root_guardian = LocalActorRef {
            inner: inner.into(),
        };
        deferred_spawns.push(DeferredSpawn::new(root_guardian.clone().into(), mailbox, root_props));
        let (system_guardian, deferred) = root_guardian
            .attach_child(Props::create(|_| SystemGuardian), Some("system".to_string()), false)?;
        deferred.into_foreach(|d| deferred_spawns.push(d));
        let system_guardian = system_guardian.local().unwrap();
        let (user_guardian, deferred) = root_guardian
            .attach_child(Props::create(|_| UserGuardian), Some("user".to_string()), false)?;
        deferred.into_foreach(|d| deferred_spawns.push(d));
        let user_guardian = user_guardian.local().unwrap();
        let inner = crate::actor::dead_letter_ref::Inner {
            system: system.clone(),
            path: root_path.child("dead_letters"),
        };
        let dead_letters = DeadLetterActorRef { inner: inner.into() };
        let temp_node = root_path.child("temp");
        let inner = crate::actor::virtual_path_container::Inner {
            system: system.clone(),
            path: temp_node.clone(),
            parent: root_guardian.clone().into(),
            children: Arc::new(DashMap::with_hasher(ahash::RandomState::new())),
        };
        let temp_container = VirtualPathContainer {
            inner: inner.into(),
        };
        let provider = LocalActorRefProvider {
            root_path: root_path.into(),
            root_guardian,
            user_guardian: user_guardian.clone(),
            system_guardian: system_guardian.clone(),
            extra_names: DashMap::with_hasher(ahash::RandomState::new()),
            dead_letters: dead_letters.into(),
            temp_number: AtomicI64::new(0),
            temp_node,
            temp_container,
        };
        Ok((provider, deferred_spawns))
    }
}

impl TActorRefProvider for LocalActorRefProvider {
    fn root_guardian(&self) -> &LocalActorRef {
        &self.root_guardian
    }

    fn root_guardian_at(&self, address: &Address) -> ActorRef {
        if self.root_path.address() == *address {
            self.root_guardian.clone().into()
        } else {
            self.dead_letters.clone()
        }
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

    fn spawn_actor(&self, props: Props, supervisor: &ActorRef) -> anyhow::Result<ActorRef> {
        supervisor.local().unwrap().attach_child(props, None, true).map(|(actor, _)| actor)
    }

    fn resolve_actor_ref_of_path(&self, path: &ActorPath) -> ActorRef {
        if path.address() == self.root_path().address() {
            //TODO opt
            let elements = path.elements();
            let iter = &mut elements.iter().map(|e| e.as_str()) as &mut dyn Iterator<Item=&str>;
            let mut iter = iter.peekable();
            match iter.peek() {
                Some(peek) if *peek == "temp" => {
                    iter.next();
                    TActorRef::get_child(&self.temp_container, &mut iter).unwrap_or_else(|| self.dead_letters().clone())
                }
                Some(peek) if *peek == "dead_letters" => {
                    self.dead_letters.clone()
                }
                Some(peek) if self.extra_names.contains_key(&**peek) => {
                    self.extra_names.get(&**peek).map(|r| r.value().clone()).unwrap_or_else(|| self.dead_letters().clone())
                }
                _ => {
                    self.root_guardian()
                        .get_child(&mut iter)
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

impl Into<ActorRefProvider> for LocalActorRefProvider {
    fn into(self) -> ActorRefProvider {
        ActorRefProvider::new(self)
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

use ahash::RandomState;
use anyhow::anyhow;
use dashmap::DashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use tokio::sync::broadcast::{channel, Receiver, Sender};

use actor_derive::AsAny;

use crate::actor::actor_system::{ActorSystem, Settings};
use crate::actor::address::{Address, Protocol};
use crate::actor::props::Props;
use crate::actor::root_guardian::RootGuardian;
use crate::actor::system_guardian::SystemGuardian;
use crate::actor::user_guardian::UserGuardian;
use crate::actor_path::root_actor_path::RootActorPath;
use crate::actor_path::{ActorPath, TActorPath};
use crate::actor_ref::dead_letter_ref::DeadLetterActorRef;
use crate::actor_ref::ignore_ref::IgnoreActorRef;
use crate::actor_ref::local_ref::LocalActorRef;
use crate::actor_ref::virtual_path_container::VirtualPathContainer;
use crate::actor_ref::{ActorRef, TActorRef};
use crate::config::akka::Akka;
use crate::ext::base64;
use crate::provider::provider::{ActorSpawn, Provider};
use crate::provider::{cast_self_to_dyn, ActorRefProvider, TActorRefProvider};
use crate::{local, REFERENCE};

#[derive(Debug, AsAny)]
pub struct LocalActorRefProvider {
    settings: Settings,
    root_path: ActorPath,
    root_guardian: LocalActorRef,
    user_guardian: LocalActorRef,
    system_guardian: LocalActorRef,
    extra_names: DashMap<String, ActorRef, RandomState>,
    dead_letters: ActorRef,
    ignore_ref: ActorRef,
    temp_number: AtomicI64,
    temp_node: ActorPath,
    temp_container: VirtualPathContainer,
    termination_tx: Sender<()>,
    pub config: Akka,
}

impl LocalActorRefProvider {
    pub fn new(mut settings: Settings) -> anyhow::Result<Provider<Self>> {
        let cfg = config::Config::builder()
            .add_source(config::File::from_str(&REFERENCE, config::FileFormat::Json))
            .add_source(settings.cfg)
            .build()?;
        settings.cfg = cfg;
        let actor_config: Akka = settings.cfg.get("akka")?;
        let mut actor_spawns = vec![];
        let (termination_tx, _) = channel(1);
        //TODO address
        let root_path = RootActorPath::new(Address::new(Protocol::Akka, "test", None), "/");
        let termination_tx_clone = termination_tx.clone();
        let root_props = Props::new(move || Ok(RootGuardian::new(termination_tx_clone)));
        let mailbox = &actor_config.actor.mailbox["default_mailbox"];
        let (sender, mailbox) = root_props.build_mailbox(mailbox)?;
        let (root_guardian, signal_rx) = LocalActorRef::new(root_path.clone().into(), sender, None);
        let root_spawn =
            ActorSpawn::new(root_props, root_guardian.clone().into(), signal_rx, mailbox);
        actor_spawns.push(root_spawn);
        let default_mailbox_cfg = actor_config
            .actor
            .mailbox
            .get("default_mailbox")
            .ok_or(anyhow!("default_mailbox not found"))?;
        let system_spawn = root_guardian.attach_child_deferred(
            Props::new(|| Ok(SystemGuardian)),
            Some("system".to_string()),
            Some(ActorPath::undefined_uid()),
            default_mailbox_cfg,
        )?;
        let system_guardian = local!(system_spawn.myself).clone();
        actor_spawns.push(system_spawn);
        let user_spawn = root_guardian.attach_child_deferred(
            Props::new(|| Ok(UserGuardian)),
            Some("user".to_string()),
            Some(ActorPath::undefined_uid()),
            default_mailbox_cfg,
        )?;
        let user_guardian = local!(user_spawn.myself).clone();
        actor_spawns.push(user_spawn);
        let dead_letters = DeadLetterActorRef::new(root_path.child("dead_letters"));
        let temp_node = root_path.child("temp");
        let temp_container =
            VirtualPathContainer::new(temp_node.clone(), root_guardian.clone().into());
        let local = Self {
            settings,
            root_path: root_path.into(),
            root_guardian,
            user_guardian: user_guardian.clone(),
            system_guardian: system_guardian.clone(),
            extra_names: DashMap::with_hasher(RandomState::new()),
            dead_letters: dead_letters.into(),
            ignore_ref: IgnoreActorRef::new().into(),
            temp_number: AtomicI64::new(0),
            temp_node,
            temp_container,
            termination_tx,
            config: actor_config,
        };
        Ok(Provider::new(local, actor_spawns))
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }
}

impl TActorRefProvider for LocalActorRefProvider {
    fn settings(&self) -> &Settings {
        &self.settings
    }

    fn root_guardian(&self) -> &LocalActorRef {
        &self.root_guardian
    }

    fn root_guardian_at(&self, address: &Address) -> ActorRef {
        if self.root_path.address() == address {
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

    fn temp_path_of_prefix(&self, prefix: Option<&str>) -> ActorPath {
        let mut builder = String::new();
        let prefix_is_none_or_empty = prefix.map(|p| p.is_empty()).unwrap_or(true);
        if !prefix_is_none_or_empty {
            builder.push_str(prefix.unwrap());
        }
        builder.push_str("$");
        let builder = base64(self.temp_number.fetch_add(1, Ordering::Relaxed), builder);
        self.temp_node.child(&builder)
    }

    fn temp_container(&self) -> ActorRef {
        self.temp_container.clone().into()
    }

    fn register_temp_actor(&self, actor: ActorRef, path: &ActorPath) {
        assert_eq!(
            path.parent(),
            self.temp_node,
            "cannot register_temp_actor() with anything not obtained from temp_path()"
        );
        self.temp_container
            .add_child(path.name().to_string(), actor);
    }

    fn unregister_temp_actor(&self, path: &ActorPath) {
        assert_eq!(
            path.parent(),
            self.temp_node,
            "cannot unregister_temp_actor() with anything not obtained from temp_path()"
        );
        self.temp_container.remove_child(path.name());
    }

    fn spawn_actor(
        &self,
        props: Props,
        supervisor: &ActorRef,
        system: ActorSystem,
    ) -> anyhow::Result<ActorRef> {
        local!(supervisor).attach_child(props, system, None, None)
    }

    fn resolve_actor_ref_of_path(&self, path: &ActorPath) -> ActorRef {
        if path.address() == self.root_path().address() {
            //TODO opt
            let elements = path.elements();
            let iter = &mut elements.iter().map(|e| e.as_str()) as &mut dyn Iterator<Item = &str>;
            let mut iter = iter.peekable();
            match iter.peek() {
                Some(peek) if *peek == "temp" => {
                    iter.next();
                    TActorRef::get_child(&self.temp_container, &mut iter)
                        .unwrap_or_else(|| self.dead_letters.clone())
                }
                Some(peek) if *peek == "dead_letters" => self.dead_letters.clone(),
                Some(peek) if self.extra_names.contains_key(&**peek) => self
                    .extra_names
                    .get(&**peek)
                    .map(|r| r.value().clone())
                    .unwrap_or_else(|| self.dead_letters.clone()),
                _ => self
                    .root_guardian()
                    .get_child(&mut iter)
                    .unwrap_or_else(|| self.dead_letters.clone()),
            }
        } else {
            self.dead_letters.clone()
        }
    }

    fn dead_letters(&self) -> &dyn TActorRef {
        &**self.dead_letters
    }

    fn ignore_ref(&self) -> &dyn TActorRef {
        &**self.ignore_ref
    }

    fn termination_rx(&self) -> Receiver<()> {
        self.termination_tx.subscribe()
    }

    fn as_provider(&self, name: &str) -> Option<&dyn TActorRefProvider> {
        cast_self_to_dyn(name, self)
    }
}

impl Into<ActorRefProvider> for LocalActorRefProvider {
    fn into(self) -> ActorRefProvider {
        ActorRefProvider::new(self)
    }
}

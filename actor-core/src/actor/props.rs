use std::fmt::{Debug, Formatter};

use anyhow::anyhow;
use tokio::runtime::Handle;
use tokio::sync::mpsc::channel;

use crate::Actor;
use crate::actor::actor_system::ActorSystem;
use crate::actor::context::ActorContext;
use crate::actor::mailbox::{Mailbox, MailboxSender};
use crate::actor_ref::ActorRef;
use crate::cell::runtime::ActorRuntime;
use crate::config::mailbox::SYSTEM_MAILBOX_SIZE;
use crate::ext::type_name_of;

type ActorSpawner = Box<dyn FnOnce(ActorRef, Mailbox, ActorSystem, Option<Handle>) -> anyhow::Result<()> + Send>;

pub struct Props {
    pub(crate) actor_name: &'static str,
    pub(crate) spawner: ActorSpawner,
    pub(crate) handle: Option<Handle>,
    pub(crate) mailbox: Option<String>,
}

impl Debug for Props {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_struct("Props")
            .field("actor_name", &self.actor_name)
            .field("handle", &self.handle)
            .field("mailbox", &self.mailbox)
            .finish_non_exhaustive()
    }
}

impl Props {
    pub fn new<F, A>(func: F) -> Self
        where
            F: FnOnce() -> anyhow::Result<A> + Send + 'static,
            A: Actor {
        let actor_name = type_name_of::<A>();
        let spawner = move |myself: ActorRef, mailbox: Mailbox, system: ActorSystem, handle: Option<Handle>| {
            let context = ActorContext::new(myself, system, handle.clone());
            let handle = handle.unwrap_or(context.system.handle().clone());
            let actor = func()?;
            let runtime = ActorRuntime { actor, context, mailbox };
            handle.spawn(runtime.run());
            Ok::<_, anyhow::Error>(())
        };
        Self {
            actor_name,
            spawner: Box::new(spawner),
            handle: None,
            mailbox: None,
        }
    }

    pub fn new_with_ctx<F, A>(func: F) -> Self
        where
            F: FnOnce(&mut ActorContext) -> anyhow::Result<A> + Send + 'static,
            A: Actor {
        let actor_name = type_name_of::<A>();
        let spawner = move |myself: ActorRef, mailbox: Mailbox, system: ActorSystem, handle: Option<Handle>| {
            let mut context = ActorContext::new(myself, system, handle.clone());
            let handle = handle.unwrap_or(context.system.handle().clone());
            let actor = func(&mut context)?;
            let runtime = ActorRuntime { actor, context, mailbox };
            handle.spawn(runtime.run());
            Ok::<_, anyhow::Error>(())
        };
        Self {
            actor_name,
            spawner: Box::new(spawner),
            handle: None,
            mailbox: None,
        }
    }

    pub(crate) fn mailbox(&self, system: &ActorSystem) -> anyhow::Result<(MailboxSender, Mailbox)> {
        let core_config = system.core_config();
        let mailbox_name = self.mailbox.as_ref().map(|m| m.as_str()).unwrap_or("default");
        let mailbox = core_config.mailbox.get(mailbox_name).ok_or(anyhow!("mailbox {} config not found", mailbox_name))?;
        let (m_tx, m_rx) = channel(mailbox.mailbox_capacity);
        let (s_tx, s_rx) = channel(SYSTEM_MAILBOX_SIZE);
        let sender = MailboxSender {
            message: m_tx,
            system: s_tx,
        };
        let mailbox = Mailbox {
            message: m_rx,
            system: s_rx,
            throughput: mailbox.throughput,
            stash_capacity: mailbox.stash_capacity,
        };
        Ok((sender, mailbox))
    }

    pub fn with_mailbox(&mut self, mailbox: impl Into<String>) -> &mut Self {
        self.mailbox = Some(mailbox.into());
        self
    }

    pub fn spawn(self, myself: ActorRef, mailbox: Mailbox, system: ActorSystem) -> anyhow::Result<()> {
        (self.spawner)(myself, mailbox, system, self.handle)
    }
}

pub trait DeferredSpawn {
    fn spawn(self: Box<Self>, system: ActorSystem) -> anyhow::Result<()>;
}

pub struct ActorDeferredSpawn {
    pub actor_ref: ActorRef,
    pub mailbox: Mailbox,
    pub spawner: ActorSpawner,
    pub handle: Option<Handle>,
}

impl ActorDeferredSpawn {
    pub fn new(actor_ref: ActorRef, mailbox: Mailbox, spawner: ActorSpawner, handle: Option<Handle>) -> Self {
        Self {
            actor_ref,
            mailbox,
            spawner,
            handle,
        }
    }
}

impl DeferredSpawn for ActorDeferredSpawn {
    fn spawn(self: Box<Self>, system: ActorSystem) -> anyhow::Result<()> {
        let Self { actor_ref, mailbox, spawner, handle } = *self;
        spawner(actor_ref, mailbox, system, handle)
    }
}

pub struct FuncDeferredSpawn {
    func: Box<dyn FnOnce(ActorSystem) -> anyhow::Result<()>>,
}

impl FuncDeferredSpawn {
    pub fn new<F>(f: F) -> Self where F: FnOnce(ActorSystem) -> anyhow::Result<()> + 'static {
        Self {
            func: Box::new(f),
        }
    }
}

impl DeferredSpawn for FuncDeferredSpawn {
    fn spawn(self: Box<Self>, system: ActorSystem) -> anyhow::Result<()> {
        let Self { func } = *self;
        func(system)?;
        Ok(())
    }
}

pub struct PropsBuilder<Arg> {
    pub name: &'static str,
    pub builder: Box<dyn Fn(Arg) -> Props + Send>,
}

impl<Arg> PropsBuilder<Arg> {
    pub fn new<A, Builder>(builder: Builder) -> Self
        where
            Builder: Fn(Arg) -> Props + Send + 'static,
            A: Actor {
        Self {
            name: type_name_of::<A>(),
            builder: Box::new(builder),
        }
    }

    pub fn props(&self, arg: Arg) -> Props {
        (self.builder)(arg)
    }
}

impl<Arg> Debug for PropsBuilder<Arg> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PropsBuilder")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

pub struct PropsBuilderSync<Arg> {
    pub name: &'static str,
    pub builder: Box<dyn Fn(Arg) -> Props + Send + Sync>,
}

impl<Arg> PropsBuilderSync<Arg> {
    pub fn new<A, Builder>(builder: Builder) -> Self
        where
            Builder: Fn(Arg) -> Props + Send + Sync + 'static,
            A: Actor {
        Self {
            name: type_name_of::<A>(),
            builder: Box::new(builder),
        }
    }

    pub fn props(&self, arg: Arg) -> Props {
        (self.builder)(arg)
    }
}

impl<A> Debug for PropsBuilderSync<A> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PropsBuilderSync")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}
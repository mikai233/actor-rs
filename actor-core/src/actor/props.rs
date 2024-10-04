use std::any::type_name;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;

use anyhow::anyhow;
use tokio::sync::mpsc::channel;

use crate::actor::actor_system::{ActorSystem, WeakActorSystem};
use crate::actor::context::ActorContext;
use crate::actor::mailbox::{Mailbox, MailboxSender};
use crate::actor::Actor;
use crate::actor_ref::actor_ref_factory::ActorRefFactory;
use crate::actor_ref::ActorRef;
use crate::cell::runtime::ActorRuntime;
use crate::config::mailbox::SYSTEM_MAILBOX_SIZE;
use crate::provider::local_provider::LocalActorRefProvider;

type ActorSpawner = Box<dyn FnOnce(ActorRef, Mailbox, ActorSystem) -> anyhow::Result<()> + Send>;

pub struct Props {
    pub(crate) actor_name: &'static str,
    pub(crate) spawner: ActorSpawner,
    pub(crate) mailbox: Option<String>,
}

impl Debug for Props {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_struct("Props")
            .field("actor_name", &self.actor_name)
            .field("mailbox", &self.mailbox)
            .finish_non_exhaustive()
    }
}

impl Props {
    pub fn new<F, A>(func: F) -> Self
    where
        F: FnOnce() -> anyhow::Result<A> + Send + 'static,
        A: Actor,
    {
        let spawner = move |myself: ActorRef, mailbox: Mailbox, system: ActorSystem| {
            let actor = func()?;
            let mut context = ActorContext::new(myself, system);
            let receive = actor.receive();
            context.r#become(
                move |actor, ctx, message, sender| receive.receive(actor, ctx, message, sender),
                false,
            );
            let runtime = ActorRuntime {
                actor,
                context,
                mailbox,
            };
            Self::run_actor(runtime)?;
            Ok::<_, anyhow::Error>(())
        };
        Self {
            actor_name: type_name::<A>(),
            spawner: Box::new(spawner),
            mailbox: None,
        }
    }

    pub fn new_with_ctx<F, A>(func: F) -> Self
    where
        F: FnOnce(&mut ActorContext) -> anyhow::Result<A> + Send + 'static,
        A: Actor,
    {
        let actor_name = type_name::<A>();
        let spawner = move |myself: ActorRef, mailbox: Mailbox, system: ActorSystem| {
            let mut context = ActorContext::new(myself, system);
            let actor = func(&mut context)?;
            let runtime = ActorRuntime {
                actor,
                context,
                mailbox,
            };
            Self::run_actor(runtime)?;
            Ok::<_, anyhow::Error>(())
        };
        Self {
            actor_name,
            spawner: Box::new(spawner),
            mailbox: None,
        }
    }

    pub(crate) fn mailbox(&self, system: &ActorSystem) -> anyhow::Result<(MailboxSender, Mailbox)> {
        let provider = system.provider();
        let local = provider
            .downcast_ref::<LocalActorRefProvider>()
            .ok_or(anyhow!("LocalActorRefProvider not found"))?;
        let mailbox = &local
            .settings()
            .actor
            .mailbox
            .get("default")
            .ok_or(anyhow!("akka.actor.mailbox default config not found"))?;
        //TODO: mailbox config
        let (m_tx, m_rx) = channel(mailbox.mailbox_capacity.unwrap_or(1000000));
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

    pub(crate) fn spawn(
        self,
        myself: ActorRef,
        mailbox: Mailbox,
        system: WeakActorSystem,
    ) -> anyhow::Result<()> {
        (self.spawner)(myself, mailbox, system.upgrade()?)
    }

    #[cfg(feature = "tokio-tracing")]
    pub(crate) fn run_actor<A>(rt: ActorRuntime<A>) -> anyhow::Result<()>
    where
        A: Actor,
    {
        tokio::task::Builder::new()
            .name(type_name::<A>())
            .spawn(rt.run())?;
        Ok(())
    }

    #[cfg(not(feature = "tokio-tracing"))]
    pub(crate) fn run_actor<A>(rt: ActorRuntime<A>) -> anyhow::Result<()>
    where
        A: Actor,
    {
        tokio::spawn(rt.run());
        Ok(())
    }
}

pub trait DeferredSpawn {
    fn spawn(self: Box<Self>, system: ActorSystem) -> anyhow::Result<()>;
}

pub struct ActorDeferredSpawn {
    pub actor_ref: ActorRef,
    pub mailbox: Mailbox,
    pub spawner: ActorSpawner,
}

impl ActorDeferredSpawn {
    pub fn new(actor_ref: ActorRef, mailbox: Mailbox, spawner: ActorSpawner) -> Self {
        Self {
            actor_ref,
            mailbox,
            spawner,
        }
    }
}

impl DeferredSpawn for ActorDeferredSpawn {
    fn spawn(self: Box<Self>, system: ActorSystem) -> anyhow::Result<()> {
        let Self {
            actor_ref,
            mailbox,
            spawner,
        } = *self;
        spawner(actor_ref, mailbox, system)
    }
}

impl Debug for ActorDeferredSpawn {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ActorDeferredSpawn")
            .field("actor_ref", &self.actor_ref)
            .field("mailbox", &self.mailbox)
            .finish_non_exhaustive()
    }
}

pub struct FuncDeferredSpawn {
    func: Box<dyn FnOnce(ActorSystem) -> anyhow::Result<()>>,
}

impl FuncDeferredSpawn {
    pub fn new<F>(f: F) -> Self
    where
        F: FnOnce(ActorSystem) -> anyhow::Result<()> + 'static,
    {
        Self { func: Box::new(f) }
    }
}

impl DeferredSpawn for FuncDeferredSpawn {
    fn spawn(self: Box<Self>, system: ActorSystem) -> anyhow::Result<()> {
        let Self { func } = *self;
        func(system)?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct PropsBuilder<Arg> {
    pub name: &'static str,
    pub builder: Arc<dyn Fn(Arg) -> Props + Send + Sync>,
}

impl<Arg> PropsBuilder<Arg> {
    pub fn new<A, Builder>(builder: Builder) -> Self
    where
        Builder: Fn(Arg) -> anyhow::Result<A> + Send + Sync + 'static,
        Arg: Send + 'static,
        A: Actor,
    {
        let builder = Arc::new(builder);
        let props_builder = move |arg: Arg| {
            let builder = builder.clone();
            Props::new(move || builder(arg))
        };

        Self {
            name: type_name::<A>(),
            builder: Arc::new(props_builder),
        }
    }

    pub fn new_wit_ctx<A, Builder>(builder: Builder) -> Self
    where
        Builder: Fn(&mut ActorContext, Arg) -> anyhow::Result<A> + Send + Sync + 'static,
        Arg: Send + 'static,
        A: Actor,
    {
        let builder = Arc::new(builder);
        let props_builder = move |arg: Arg| {
            let builder = builder.clone();
            Props::new_with_ctx(move |ctx| builder(ctx, arg))
        };

        Self {
            name: type_name::<A>(),
            builder: Arc::new(props_builder),
        }
    }

    pub fn props(&self, arg: Arg) -> Props {
        (self.builder)(arg)
    }
}

impl<A> Debug for PropsBuilder<A> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PropsBuilderSync")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

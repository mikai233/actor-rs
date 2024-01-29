use std::fmt::{Debug, Formatter};
use std::sync::Arc;

use anyhow::anyhow;
use tokio::runtime::Handle;
use tokio::sync::mpsc::channel;

use crate::Actor;
use crate::actor::actor_ref::ActorRef;
use crate::actor::actor_system::ActorSystem;
use crate::actor::context::ActorContext;
use crate::actor::mailbox::{Mailbox, MailboxSender};
use crate::cell::runtime::ActorRuntime;
use crate::config::mailbox::SYSTEM_MAILBOX_SIZE;
use crate::routing::router_config::RouterConfig;

pub type Spawner = Arc<Box<dyn Fn(ActorRef, Mailbox, ActorSystem, Props) + Send + Sync + 'static>>;

#[derive(Clone)]
pub struct Props {
    pub(crate) spawner: Spawner,
    pub(crate) handle: Option<Handle>,
    pub(crate) router_config: Option<RouterConfig>,
    pub(crate) mailbox: Option<String>,
}

impl Debug for Props {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_struct("Props")
            .field("spawner", &"..")
            .field("handle", &self.handle)
            .field("router_config", &self.router_config)
            .field("mailbox", &self.mailbox)
            .finish()
    }
}

impl Props {
    pub fn create<F, A>(f: F) -> Self where F: Fn(&mut ActorContext) -> A + Send + Sync + 'static, A: Actor {
        let spawner = move |myself: ActorRef, mailbox: Mailbox, system: ActorSystem, props: Props| {
            let handle = props.handle.clone();
            let mut context = ActorContext::new(myself, system, props);
            let system = context.system.clone();
            let actor = f(&mut context);
            let runtime = ActorRuntime {
                actor,
                context,
                mailbox,
            };
            match handle {
                None => {
                    system.user_rt().spawn(runtime.run());
                }
                Some(handle) => {
                    handle.spawn(runtime.run());
                }
            }
        };
        Self {
            spawner: Arc::new(Box::new(spawner)),
            handle: None,
            router_config: None,
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

    pub fn with_router(&self, r: Option<RouterConfig>) -> Props {
        let mut props = self.clone();
        props.router_config = r;
        props
    }

    pub fn router_config(&self) -> Option<&RouterConfig> {
        self.router_config.as_ref()
    }

    pub fn with_mailbox(&mut self, mailbox: impl Into<String>) -> &mut Self {
        self.mailbox = Some(mailbox.into());
        self
    }
}

pub trait DeferredSpawn {
    fn spawn(self: Box<Self>, system: ActorSystem) -> anyhow::Result<()>;
}

pub struct ActorDeferredSpawn {
    pub actor_ref: ActorRef,
    pub mailbox: Mailbox,
    pub props: Props,
}

impl ActorDeferredSpawn {
    pub fn new(actor_ref: ActorRef, mailbox: Mailbox, props: Props) -> Self {
        Self {
            actor_ref,
            mailbox,
            props,
        }
    }
}

impl DeferredSpawn for ActorDeferredSpawn {
    fn spawn(self: Box<Self>, system: ActorSystem) -> anyhow::Result<()> {
        let Self { actor_ref, mailbox, props } = *self;
        let spawner = props.spawner.clone();
        spawner(actor_ref, mailbox, system, props);
        Ok(())
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
use std::sync::Arc;

use tokio::sync::mpsc::channel;

use crate::Actor;
use crate::actor::actor_ref::ActorRef;
use crate::actor::actor_system::ActorSystem;
use crate::actor::context::ActorContext;
use crate::actor::mailbox::{Mailbox, MailboxSender};
use crate::cell::runtime::ActorRuntime;
use crate::routing::router_config::RouterConfig;

pub type Spawner = Arc<Box<dyn Fn(ActorRef, Mailbox, ActorSystem, Props) + Send + Sync + 'static>>;

#[derive(Clone)]
pub struct Props {
    pub(crate) spawner: Spawner,
    pub(crate) router_config: Option<RouterConfig>,
    pub(crate) mailbox_size: usize,
    pub(crate) system_size: usize,
    pub(crate) throughput: usize,
}

impl Props {
    pub fn create<F, A>(f: F) -> Self where F: Fn(&mut ActorContext) -> A + Send + Sync + 'static, A: Actor {
        let spawn_fn = move |myself: ActorRef, mailbox: Mailbox, system: ActorSystem, props: Props| {
            let mut context = ActorContext::new(myself, system);
            let system = context.system.clone();
            let actor = f(&mut context);
            let runtime = ActorRuntime {
                actor,
                context,
                mailbox,
                props,
            };
            system.spawn(runtime.run());
        };
        Self {
            spawner: Arc::new(Box::new(spawn_fn)),
            router_config: None,
            mailbox_size: 10000,
            system_size: 10000,
            throughput: 10,
        }
    }
    pub(crate) fn mailbox(&self) -> (MailboxSender, Mailbox) {
        let (m_tx, m_rx) = channel(self.mailbox_size);
        let (s_tx, s_rx) = channel(self.system_size);
        let sender = MailboxSender {
            message: m_tx,
            system: s_tx,
        };
        let mailbox = Mailbox {
            message: m_rx,
            system: s_rx,
            throughput: self.throughput,
        };
        (sender, mailbox)
    }

    pub fn with_router(&self, r: Option<RouterConfig>) -> Props {
        let mut props = self.clone();
        props.router_config = r;
        props
    }

    pub fn router_config(&self) -> Option<&RouterConfig> {
        self.router_config.as_ref()
    }
}

pub trait DeferredSpawn {
    fn spawn(self: Box<Self>, system: ActorSystem);
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
    fn spawn(self: Box<Self>, system: ActorSystem) {
        let Self { actor_ref, mailbox, props } = *self;
        let spawner = props.spawner.clone();
        spawner(actor_ref, mailbox, system, props);
    }
}

pub struct FuncDeferredSpawn {
    func: Box<dyn FnOnce(ActorSystem)>,
}

impl FuncDeferredSpawn {
    pub fn new<F>(f: F) -> Self where F: FnOnce(ActorSystem) + 'static {
        Self {
            func: Box::new(f),
        }
    }
}

impl DeferredSpawn for FuncDeferredSpawn {
    fn spawn(self: Box<Self>, system: ActorSystem) {
        let Self { func } = *self;
        func(system);
    }
}
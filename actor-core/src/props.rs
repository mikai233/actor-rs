use std::sync::Arc;

use tokio::sync::mpsc::channel;

use crate::Actor;
use crate::actor::actor_ref::ActorRef;
use crate::actor::actor_system::ActorSystem;
use crate::actor::context::ActorContext;
use crate::cell::runtime::ActorRuntime;
use crate::actor::mailbox::{Mailbox, MailboxSender};
use crate::routing::router_config::{RouterConfig, TRouterConfig};

#[derive(Clone)]
pub struct Props {
    pub(crate) spawner: Arc<Box<dyn Fn(ActorRef, Mailbox, ActorSystem) + Send + Sync + 'static>>,
    pub(crate) router_config: Option<Arc<RouterConfig>>,
    pub(crate) mailbox_size: usize,
    pub(crate) system_size: usize,
    pub(crate) throughput: usize,
}

impl Props {
    pub fn create<F, A>(f: F) -> Self where F: Fn(&mut ActorContext) -> A + 'static + Send + Sync + 'static, A: Actor {
        let spawn_fn = move |myself: ActorRef, mailbox: Mailbox, system: ActorSystem| {
            let mut context = ActorContext::new(myself, system);
            let system = context.system.clone();
            let actor = f(&mut context);
            let runtime = ActorRuntime {
                actor,
                context,
                mailbox,
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

    pub fn with_router<R>(&self, r: R) -> Props where R: TRouterConfig {
        todo!()
        // let mut props = Clone::clone(self);
        // props.router_config = Some(Box::new(r));
        // props
    }

    pub fn router_config(&self) -> Option<&Arc<RouterConfig>> {
        self.router_config.as_ref()
    }
}
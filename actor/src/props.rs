use tokio::sync::mpsc::channel;

use crate::Actor;
use crate::net::mailbox::{Mailbox, MailboxSender};
use crate::routing::router_config::RouterConfig;

#[derive(Clone)]
pub struct Props<T> where T: Actor {
    pub(crate) arg: T::A,
    pub(crate) router_config: Option<Box<dyn RouterConfig>>,
    pub(crate) mailbox_size: usize,
    pub(crate) system_size: usize,
    pub(crate) throughput: usize,
}

impl<T> Props<T> where T: Actor {
    pub fn create(arg: T::A) -> Props<T> {
        Self {
            arg,
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

    pub fn with_router<R>(r: R) -> Props<T> where R: RouterConfig {
        todo!()
    }

    pub fn router_config(&self) -> Option<&Box<dyn RouterConfig>> {
        self.router_config.as_ref()
    }
}

pub fn props<T>(arg: T::A) -> Props<T> where T: Actor {
    Props::create(arg)
}

pub fn noarg_props<T>() -> Props<T> where T: Actor<A=()> {
    Props::create(())
}
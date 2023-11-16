use std::fmt::Debug;

use crate::net::mailbox::{Mailbox, MailboxSender};

#[derive(Debug, Copy, Clone)]
pub struct Props {
    pub mailbox: usize,
    pub throughput: usize,
}

impl Props {
    pub(crate) fn mailbox(&self) -> (MailboxSender, Mailbox) {
        let (m_tx, m_rx) = tokio::sync::mpsc::channel(self.mailbox);
        let (s_tx, s_rx) = tokio::sync::mpsc::channel(1000);
        let sender = MailboxSender {
            message: m_tx,
            system: s_tx,
        };
        let mailbox = Mailbox {
            message: m_rx,
            system: s_rx,
        };
        (sender, mailbox)
    }
}

impl Default for Props {
    fn default() -> Self {
        Self {
            mailbox: 6000,
            throughput: 10,
        }
    }
}
use tokio::sync::mpsc::{Receiver, Sender};

use crate::cell::envelope::Envelope;

pub struct Mailbox {
    pub(crate) message: Receiver<Envelope>,
    pub(crate) system: Receiver<Envelope>,
    pub(crate) throughput: usize,
    pub(crate) stash_capacity: Option<usize>,
}

impl Mailbox {
    pub(crate) fn close(&mut self) {
        while let Ok(_) = self.message.try_recv() {}
        while let Ok(_) = self.system.try_recv() {}
        self.message.close();
        self.system.close();
    }
}

#[derive(Debug, Clone)]
pub(crate) struct MailboxSender {
    pub(crate) message: Sender<Envelope>,
    pub(crate) system: Sender<Envelope>,
}
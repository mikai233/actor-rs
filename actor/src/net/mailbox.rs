use tokio::sync::mpsc::{Receiver, Sender};

use crate::cell::envelope::Envelope;

pub(crate) struct Mailbox {
    pub(crate) message: Receiver<Envelope>,
    pub(crate) system: Receiver<Envelope>,
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
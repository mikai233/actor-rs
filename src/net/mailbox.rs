use tokio::sync::mpsc::{Receiver, Sender};

use crate::cell::envelope::Envelope;
use crate::message::ActorRemoteSystemMessage;

pub(crate) struct Mailbox {
    pub(crate) message: Receiver<Envelope>,
    pub(crate) signal: Receiver<Envelope>,
}

impl Mailbox {
    pub(crate) fn close(&mut self) {
        while let Ok(_) = self.message.try_recv() {}
        while let Ok(_) = self.signal.try_recv() {}
        self.message.close();
        self.signal.close();
    }
}

#[derive(Debug, Clone)]
pub(crate) struct MailboxSender {
    pub(crate) message: Sender<Envelope>,
    pub(crate) signal: Sender<Envelope>,
}
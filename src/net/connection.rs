use std::fmt::{Debug, Formatter};
use std::net::SocketAddr;

use tokio::sync::mpsc::error::SendError;

type ConnectionSender = tokio::sync::mpsc::UnboundedSender<ConnectionMessage>;

#[derive(Debug)]
pub enum ConnectionMessage {
    Frame(Vec<u8>),
    Close,
}

#[derive(Clone)]
pub struct Connection {
    pub addr: SocketAddr,
    pub sender: ConnectionSender,
}

impl Debug for Connection {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Connection")
            .field("addr", &self.addr)
            .field("sender", &"..")
            .finish()
    }
}

impl PartialEq for Connection {
    fn eq(&self, other: &Self) -> bool {
        self.addr == other.addr
    }
}

impl Connection {
    pub fn send(&self, frame: Vec<u8>) -> Result<(), SendError<ConnectionMessage>> {
        self.sender.send(ConnectionMessage::Frame(frame))
    }
    pub fn close(&self) -> Result<(), SendError<ConnectionMessage>> {
        self.sender.send(ConnectionMessage::Close)
    }
}
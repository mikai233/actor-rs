use std::fmt::Display;

use tokio::task::JoinHandle;

use crate::artery::connection::ConnectionTx;

#[derive(Debug)]
pub(super) enum ConnectionStatus {
    NotConnected,
    PrepareForConnect,
    Connecting(JoinHandle<()>),
    Connected(ConnectionTx),
}

impl Display for ConnectionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectionStatus::NotConnected => write!(f, "NotConnected"),
            ConnectionStatus::PrepareForConnect => write!(f, "PrepareForConnect"),
            ConnectionStatus::Connecting(_) => write!(f, "Connecting"),
            ConnectionStatus::Connected(_) => write!(f, "Connected"),
        }
    }
}

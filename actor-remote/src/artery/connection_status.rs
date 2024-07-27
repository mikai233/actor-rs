use tokio::task::JoinHandle;

use crate::artery::connection::ConnectionTx;

#[derive(Debug)]
pub(super) enum ConnectionStatus {
    NotConnected,
    PrepareForConnect,
    Connecting(JoinHandle<()>),
    Connected(ConnectionTx),
}
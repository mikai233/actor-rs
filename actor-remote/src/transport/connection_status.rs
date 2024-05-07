use tokio::task::JoinHandle;

use crate::transport::connection::ConnectionTx;

#[derive(Debug)]
pub(super) enum ConnectionStatus {
    NotConnected,
    PrepareForConnect,
    Connecting(JoinHandle<()>),
    Connected(ConnectionTx),
}
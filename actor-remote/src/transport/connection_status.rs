use tokio::task::AbortHandle;

use crate::transport::connection::ConnectionTx;

#[derive(Debug)]
pub(super) enum ConnectionStatus {
    NotConnected,
    PrepareForConnect,
    Connecting(AbortHandle),
    Connected(ConnectionTx),
}
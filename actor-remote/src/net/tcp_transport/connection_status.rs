use tokio::task::AbortHandle;

use crate::net::tcp_transport::connection::ConnectionTx;

#[derive(Debug)]
pub(super) enum ConnectionStatus {
    NotConnected,
    PrepareForConnect,
    Connecting(AbortHandle),
    Connected(ConnectionTx),
}
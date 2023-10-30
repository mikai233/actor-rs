use std::iter::repeat_with;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures::{SinkExt, StreamExt};
use futures::future::join_all;
use stubborn_io::{ReconnectOptions, StubbornTcpStream};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tokio_util::codec::Framed;
use tracing::{error, warn};

use crate::net::codec::{Packet, PacketCodec};
use crate::net::connection::{Connection, ConnectionMessage};
use crate::net::listener::ConnectionListener;

#[derive(Debug, Clone)]
pub struct TcpTransport {
    addr: SocketAddr,
    server_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
    client_handles: Arc<Mutex<Vec<JoinHandle<()>>>>,
}

impl TcpTransport {
    pub fn new(addr: SocketAddr) -> Self {
        Self {
            addr,
            server_handle: Arc::new(Mutex::new(None)),
            client_handles: Arc::new(Mutex::new(vec![])),
        }
    }
    pub async fn listen<L>(&mut self, listener: L) -> anyhow::Result<()> where L: ConnectionListener {
        let tcp_listener = TcpListener::bind(&self.addr).await?;
        let addr = self.addr.clone();
        let handle = tokio::spawn(async move {
            loop {
                match tcp_listener.accept().await {
                    Ok((stream, peer_addr)) => {
                        TcpTransport::accept_connection(stream, peer_addr, listener.clone());
                    }
                    Err(err) => {
                        warn!("{} accept connection error {:?}",addr,err);
                    }
                }
            }
        });
        let mut server_handle = self.server_handle.lock().unwrap();
        *server_handle = Some(handle);
        Ok(())
    }

    pub async fn connect<L>(&mut self, addr: SocketAddr, listener: L) -> anyhow::Result<()> where L: ConnectionListener {
        let opts = ReconnectOptions::new()
            .with_exit_if_first_connect_fails(false)
            .with_retries_generator(|| repeat_with(|| { Duration::from_secs(3) }));
        let stream = StubbornTcpStream::connect_with_options(addr, opts).await?;
        stream.set_nodelay(true)?;
        let handle = TcpTransport::accept_connection(stream, addr, listener);
        self.client_handles.lock().unwrap().push(handle);
        Ok(())
    }

    fn accept_connection<L, S>(stream: S, addr: SocketAddr, listener: L) -> JoinHandle<()> where L: ConnectionListener, S: Send + AsyncRead + AsyncWrite + Unpin + 'static {
        let mut framed = Framed::new(stream, PacketCodec);
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let connection = Connection { addr, sender: tx };
        listener.on_connected(&connection);
        let handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    inbound = framed.next() => {
                        match inbound {
                            Some(Ok(inbound)) => {
                                listener.on_recv(&connection, inbound.body);
                            }
                            Some(Err(err)) => {
                                error!("{} codec error {:?}",connection.addr,err);
                                break;
                            }
                            None => {
                                break;
                            }
                        }
                    }
                    outbound = rx.recv() => {
                        match outbound {
                            Some(message) => {
                                match message {
                                    ConnectionMessage::Frame(body) => {
                                        let packet = Packet::new(body);
                                        if let Some(err) = framed.send(packet).await.err() {
                                            error!("{} codec error {:?}",connection.addr,err);
                                            break;
                                        }
                                    }
                                    ConnectionMessage::Close => {
                                        break;
                                    }
                                }
                            }
                            None => {
                                break;
                            }
                        }
                    }
                }
            }
            listener.on_disconnected(&connection);
        });
        handle
    }

    pub fn close(&self) {
        if let Some(handle) = self.server_handle.lock().unwrap().as_ref() {
            handle.abort();
        }
    }

    pub async fn wait(&mut self) -> anyhow::Result<()> {
        if let Some(handle) = self.server_handle.lock().unwrap().take() {
            handle.await?;
        }
        let mut client_handles = self.client_handles.lock().unwrap();
        let client_handles = client_handles.drain(0..);
        join_all(client_handles).await;
        Ok(())
    }
}

#[cfg(test)]
mod transport_test {
    use std::net::SocketAddr;
    use std::time::Duration;

    use tracing::{info, Level};

    use crate::ext::init_logger;
    use crate::net::connection::Connection;
    use crate::net::listener::ConnectionListener;
    use crate::net::tcp_transport::TcpTransport;

    #[ctor::ctor]
    fn init() {
        init_logger(Level::DEBUG)
    }

    #[derive(Clone)]
    struct ServerListener;

    impl ConnectionListener for ServerListener {
        fn on_connected(&self, connection: &Connection) {
            info!("server {:?} connected",connection);
        }

        fn on_recv(&self, connection: &Connection, frame: Vec<u8>) {
            info!("server {:?} recv ping from client",connection);
            connection.send(vec![]).unwrap();
        }

        fn on_disconnected(&self, connection: &Connection) {
            info!("server {:?} disconnected",connection);
        }
    }

    #[derive(Clone)]
    struct ClientListener;

    impl ConnectionListener for ClientListener {
        fn on_connected(&self, connection: &Connection) {
            info!("client {:?} connected",connection);
            let connection = connection.clone();
            tokio::spawn(async move {
                for i in 0..10 {
                    connection.send(vec![]).unwrap();
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
                // connection.close().unwrap();
            });
        }

        fn on_recv(&self, connection: &Connection, frame: Vec<u8>) {
            info!("client {:?} recv pong from server",connection);
        }

        fn on_disconnected(&self, connection: &Connection) {
            info!("client {:?} disconnected",connection);
        }
    }

    #[ignore]
    #[tokio::test]
    async fn test_server() -> anyhow::Result<()> {
        let addr: SocketAddr = "127.0.0.1:8080".parse()?;
        let mut materializer = TcpTransport::new(addr);
        materializer.listen(ServerListener).await?;
        materializer.wait().await?;
        Ok(())
    }

    #[ignore]
    #[tokio::test]
    async fn test_client() -> anyhow::Result<()> {
        let addr: SocketAddr = "127.0.0.1:8080".parse()?;
        let mut materializer = TcpTransport::new(addr);
        materializer.connect(addr, ClientListener).await?;
        materializer.wait().await?;
        Ok(())
    }
}
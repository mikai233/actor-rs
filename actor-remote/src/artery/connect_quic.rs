use std::net::SocketAddr;

use async_trait::async_trait;
use quinn::{ClientConfig, Endpoint, SendStream};
use tokio_util::codec::FramedWrite;
use tracing::{debug, error, info};

use actor_core::actor::context::{ActorContext1, ActorContext};
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::actor_ref::ActorRefExt;
use actor_core::EmptyCodec;
use actor_core::Message;

use crate::artery::ArteryActor;
use crate::artery::codec::PacketCodec;
use crate::artery::connect_tcp_failed::ConnectFailed;
use crate::artery::connected::Connected;
use crate::artery::connection::Connection;
use crate::artery::connection_status::ConnectionStatus;

#[derive(Debug, EmptyCodec)]
pub(super) struct ConnectQuic {
    pub(super) addr: SocketAddr,
    pub(super) config: ClientConfig,
}

#[async_trait]
impl Message for ConnectQuic {
    type A = ArteryActor;

    async fn handle(self: Box<Self>, context: &mut ActorContext1, actor: &mut Self::A) -> anyhow::Result<()> {
        let Self { addr, config } = *self;
        if actor.is_connecting_or_connected(&addr) {
            debug!("ignore connect to {} because it is already connected", addr);
            return Ok(());
        }
        let myself = context.myself().clone();
        let myself_addr = context.system().address();
        let handle = context.spawn_fut(format!("connect_quic_{}", addr), async move {
            match Self::connect(addr, config).await {
                Ok(stream) => {
                    let framed = FramedWrite::new(stream, PacketCodec);
                    let (connection, tx) = Connection::new(
                        addr,
                        framed,
                        myself.clone(),
                        myself_addr,
                    );
                    connection.start();
                    myself.cast_ns(Connected { addr, tx });
                }
                Err(error) => {
                    error!("connect {} error {:?}, drop current connection", addr, error);
                    myself.cast_ns(ConnectFailed { addr });
                }
            };
        })?;
        actor.connections.insert(addr, ConnectionStatus::Connecting(handle));
        Ok(())
    }
}

impl ConnectQuic {
    async fn connect(addr: SocketAddr, config: ClientConfig) -> anyhow::Result<SendStream> {
        let mut endpoint = Endpoint::client("0.0.0.0:0".parse()?)?;
        endpoint.set_default_client_config(config);
        let connection = endpoint.connect(addr, "localhost")?.await?;
        info!("{} connected to {}", endpoint.local_addr()?, connection.remote_address());
        let stream = connection.open_uni().await?;
        Ok(stream)
    }
}
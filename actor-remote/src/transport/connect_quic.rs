use std::net::SocketAddr;

use async_trait::async_trait;
use quinn::{ClientConfig, Endpoint};
use tokio_util::codec::FramedWrite;
use tracing::info;

use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::actor_ref::ActorRefExt;
use actor_core::EmptyCodec;
use actor_core::Message;

use crate::transport::codec::PacketCodec;
use crate::transport::connected::Connected;
use crate::transport::connection::Connection;
use crate::transport::TransportActor;

#[derive(Debug, EmptyCodec)]
pub(super) struct ConnectQuic {
    pub(super) addr: SocketAddr,
    pub(super) config: ClientConfig,
}

#[async_trait]
impl Message for ConnectQuic {
    type A = TransportActor;

    async fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut Self::A) -> eyre::Result<()> {
        let Self { addr, config } = *self;
        let myself = context.myself().clone();
        let myself_addr = context.system().address();
        let mut endpoint = Endpoint::client("0.0.0.0:0".parse()?)?;
        endpoint.set_default_client_config(config);
        let connection = endpoint.connect(addr, "localhost")?.await?;
        info!("{} connected to {}", endpoint.local_addr()?, connection.remote_address());
        let stream = connection.open_uni().await?;
        let framed = FramedWrite::new(stream, PacketCodec);
        let (connection, tx) = Connection::new(
            addr,
            framed,
            myself.clone(),
            myself_addr,
        );
        connection.start();
        myself.cast_ns(Connected { addr, tx });
        Ok(())
    }
}
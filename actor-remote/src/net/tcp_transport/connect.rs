use std::iter::repeat_with;
use std::net::SocketAddr;
use std::time::Duration;

use async_trait::async_trait;
use stubborn_io::{ReconnectOptions, StubbornTcpStream};
use tokio_util::codec::Framed;
use tracing::{error, warn};

use actor_core::actor::actor_ref::ActorRefExt;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::Message;
use actor_derive::EmptyCodec;

use crate::net::codec::PacketCodec;
use crate::net::tcp_transport::connected::Connected;
use crate::net::tcp_transport::connection::Connection;
use crate::net::tcp_transport::connection_status::ConnectionStatus;
use crate::net::tcp_transport::TcpTransportActor;

#[derive(EmptyCodec)]
pub(super) struct Connect {
    pub addr: SocketAddr,
    pub opts: ReconnectOptions,
}

impl Connect {
    pub fn with_infinite_retry(addr: SocketAddr) -> Self {
        let opts = ReconnectOptions::new()
            .with_exit_if_first_connect_fails(false)
            .with_retries_generator(|| repeat_with(|| Duration::from_secs(3)));
        Self {
            addr,
            opts,
        }
    }
}

#[async_trait]
impl Message for Connect {
    type A = TcpTransportActor;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let Self { addr, opts } = *self;
        let myself = context.myself().clone();
        let handle = context.spawn_fut(async move {
            match StubbornTcpStream::connect_with_options(addr, opts).await {
                //TODO 对于非集群内的地址，以及集群节点离开后，不能再一直尝试连接
                Ok(stream) => {
                    if let Some(e) = stream.set_nodelay(true).err() {
                        warn!("connect {} set tcp nodelay error {:?}, drop current connection", addr, e);
                        return;
                    }
                    let framed = Framed::new(stream, PacketCodec);
                    let (connection, tx) = Connection::new(addr, framed, myself.clone());
                    connection.start();
                    myself.cast_ns(Connected { addr, tx });
                }
                Err(e) => {
                    error!("connect to {} error {:?}, drop current connection", addr, e);
                }
            };
        });
        actor.connections.insert(addr, ConnectionStatus::Connecting(handle));
        Ok(())
    }
}
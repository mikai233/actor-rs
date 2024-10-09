use std::iter::repeat_with;
use std::net::SocketAddr;
use std::time::Duration;

use async_trait::async_trait;
use stubborn_io::{ReconnectOptions, StubbornTcpStream};
use tokio_util::codec::FramedWrite;
use tracing::{debug, error, warn};

use actor_core::actor::context::{Context, ActorContext};
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

#[derive(EmptyCodec)]
pub(super) struct ConnectTcp {
    pub(super) addr: SocketAddr,
    pub(super) opts: ReconnectOptions,
}

impl ConnectTcp {
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
impl Message for ConnectTcp {
    type A = ArteryActor;

    async fn handle(self: Box<Self>, context: &mut Context, actor: &mut Self::A) -> anyhow::Result<()> {
        let Self { addr, opts } = *self;
        if actor.is_connecting_or_connected(&addr) {
            debug!("ignore connect to {} because it is already connected", addr);
            return Ok(());
        }
        if actor.connections.contains_key(&addr) {}
        let myself = context.myself().clone();
        let myself_addr = context.system().address();
        let handle = context.spawn_fut(format!("connect_tcp_{}", addr), async move {
            match StubbornTcpStream::connect_with_options(addr, opts).await {
                //TODO 对于非集群内的地址，以及集群节点离开后，不能再一直尝试连接
                Ok(stream) => {
                    if let Some(e) = stream.set_nodelay(true).err() {
                        warn!("connect {} set tcp nodelay error {:?}, drop current connection", addr, e);
                        return;
                    }
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
                Err(e) => {
                    error!("connect to {} error {:?}, drop current connection", addr, e);
                    myself.cast_ns(ConnectFailed { addr });
                }
            };
        })?;
        actor.connections.insert(addr, ConnectionStatus::Connecting(handle));
        Ok(())
    }
}
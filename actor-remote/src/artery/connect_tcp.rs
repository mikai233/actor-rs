use crate::artery::codec::PacketCodec;
use crate::artery::connect_tcp_failed::ConnectFailed;
use crate::artery::connected::Connected;
use crate::artery::connection::Connection;
use crate::artery::connection_status::ConnectionStatus;
use crate::artery::ArteryActor;
use actor_core::actor::behavior::Behavior;
use actor_core::actor::context::ActorContext;
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::actor_ref::{ActorRef, ActorRefExt};
use actor_core::message::handler::MessageHandler;
use actor_core::Message;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::iter::repeat_with;
use std::net::SocketAddr;
use std::time::Duration;
use stubborn_io::{ReconnectOptions, StubbornTcpStream};
use tokio_util::codec::FramedWrite;
use tracing::{debug, error, warn};

#[derive(Debug, Serialize, Deserialize, Message)]
pub(super) struct ConnectTcp {
    pub(super) addr: SocketAddr,
    pub(super) opts: ReconnectOptions,
}

impl ConnectTcp {
    pub fn with_infinite_retry(addr: SocketAddr) -> Self {
        let opts = ReconnectOptions::new()
            .with_exit_if_first_connect_fails(false)
            .with_retries_generator(|| repeat_with(|| Duration::from_secs(3)));
        Self { addr, opts }
    }
}

impl Display for ConnectTcp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ConnectTcp({})", self.addr)
    }
}

impl MessageHandler<ArteryActor> for ConnectTcp {
    fn handle(
        actor: &mut ArteryActor,
        ctx: &mut <ArteryActor as Actor>::Context,
        message: Self,
        sender: Option<ActorRef>,
        _: &Receive<ArteryActor>,
    ) -> anyhow::Result<Behavior<ArteryActor>> {
        let Self { addr, opts } = message;
        if actor.is_connecting_or_connected(&addr) {
            debug!("ignore connect to {} because it is already connected", addr);
            return Ok(());
        }
        let myself = ctx.myself().clone();
        let myself_addr = ctx.system().address();
        let handle = ctx.spawn_fut(format!("connect_tcp_{}", addr), async move {
            match StubbornTcpStream::connect_with_options(addr, opts).await {
                //TODO 对于非集群内的地址，以及集群节点离开后，不能再一直尝试连接
                Ok(stream) => {
                    if let Some(e) = stream.set_nodelay(true).err() {
                        warn!(
                            "connect {} set tcp nodelay error {:?}, drop current connection",
                            addr, e
                        );
                        return;
                    }
                    let framed = FramedWrite::new(stream, PacketCodec);
                    let (connection, tx) =
                        Connection::new(addr, framed, myself.clone(), myself_addr);
                    connection.start();
                    myself.cast_ns(Connected { addr, tx });
                }
                Err(e) => {
                    error!("connect to {} error {:?}, drop current connection", addr, e);
                    myself.cast_ns(ConnectFailed { addr });
                }
            };
        })?;
        actor
            .connections
            .insert(addr, ConnectionStatus::Connecting(handle));
        Ok(Behavior::same())
    }
}

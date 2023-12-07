use std::future::Future;
use std::iter::repeat_with;
use std::net::SocketAddr;
use std::pin::Pin;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use stubborn_io::{ReconnectOptions, StubbornTcpStream};
use tokio::sync::mpsc::error::TrySendError;
use tokio_util::codec::Framed;
use tracing::{debug, error, info, warn};

use actor_derive::EmptyCodec;

use actor_core::actor_path::TActorPath;
use actor_core::actor_ref::{ActorRef, ActorRefExt, SerializedActorRef};
use actor_core::actor_ref::TActorRef;
use actor_core::context::{ActorContext, Context};
use actor_core::Message;
use actor_core::message::IDPacket;
use actor_remote::::codec::PacketCodec;
use actor_remote::::connection::{Connection, ConnectionTx};
use actor_remote::::tcp_transport::{ConnectionSender, TransportActor};
use actor_core::system::ActorSystem;

pub struct RemoteEnvelope {
    pub packet: IDPacket,
    pub sender: Option<ActorRef>,
    pub target: ActorRef,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RemotePacket {
    pub packet: IDPacket,
    pub sender: Option<SerializedActorRef>,
    pub target: SerializedActorRef,
}

impl Into<RemotePacket> for RemoteEnvelope {
    fn into(self) -> RemotePacket {
        RemotePacket {
            packet: self.packet,
            sender: self.sender.map(|s| s.into()),
            target: self.target.into(),
        }
    }
}

#[derive(EmptyCodec)]
pub struct Connect {
    pub addr: SocketAddr,
    pub opts: ReconnectOptions,
}

impl Message for Connect {
    type A = TransportActor;

    fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
        let myself = context.myself.clone();
        context.spawn(async move {
            match StubbornTcpStream::connect_with_options(self.addr, self.opts).await {
                Ok(stream) => {
                    if let Some(e) = stream.set_nodelay(true).err() {
                        warn!("connect {} set tcp nodelay error {:?}, drop current connection", self.addr, e);
                        return;
                    }
                    let framed = Framed::new(stream, PacketCodec);
                    let (connection, tx) = Connection::new(self.addr, framed, myself.clone());
                    connection.start();
                    myself.cast(Connected { addr: self.addr, tx }, None);
                }
                Err(e) => {
                    error!("connect to {} error {:?}, drop current connection", self.addr, e);
                }
            };
        });
        Ok(())
    }
}

#[derive(EmptyCodec)]
pub struct Connected {
    pub addr: SocketAddr,
    pub tx: ConnectionTx,
}

impl Message for Connected {
    type A = TransportActor;

    fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        actor.connections.insert(self.addr, ConnectionSender::Connected(self.tx));
        info!("{} connect to {}", context.myself, self.addr);
        context.unstash_all();
        Ok(())
    }
}

#[derive(EmptyCodec)]
pub struct Disconnect {
    pub addr: SocketAddr,
}

impl Message for Disconnect {
    type A = TransportActor;

    fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        actor.connections.remove(&self.addr);
        let myself = context.myself();
        info!("{} disconnect to {}", myself, self.addr);
        Ok(())
    }
}

#[derive(EmptyCodec)]
pub struct SpawnInbound {
    pub fut: Pin<Box<dyn Future<Output=()> + Send + 'static>>,
}

impl Message for SpawnInbound {
    type A = TransportActor;

    fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
        context.spawn(self.fut);
        Ok(())
    }
}

#[derive(EmptyCodec)]
pub struct InboundMessage {
    pub packet: RemotePacket,
}

impl Message for InboundMessage {
    type A = TransportActor;

    fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let RemotePacket {
            packet,
            sender,
            target,
        } = self.packet;
        let sender = sender.map(|s| actor.resolve_actor_ref(context, s));
        let target = actor.resolve_actor_ref(context, target);
        let system: ActorSystem = target.system();
        let reg = system.registration();
        let message = reg.decode(&actor.provider, packet)?;
        target.tell(message, sender);
        Ok(())
    }
}

#[derive(EmptyCodec)]
pub struct OutboundMessage {
    pub envelope: RemoteEnvelope,
}

impl Message for OutboundMessage {
    type A = TransportActor;

    fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let addr: SocketAddr = self.envelope.target.path().address().addr.into();
        let sender = actor.connections.entry(addr).or_insert(ConnectionSender::NotConnected);
        match sender {
            ConnectionSender::NotConnected => {
                let opts = ReconnectOptions::new()
                    .with_exit_if_first_connect_fails(false)
                    .with_retries_generator(|| repeat_with(|| Duration::from_secs(3)));
                context.myself
                    .cast(Connect { addr, opts }, None);
                context.stash(OutboundMessage { envelope: self.envelope });
                debug!("message {} to {} not connected, stash current message and start connect", context.myself, addr);
                *sender = ConnectionSender::Connecting;
            }
            ConnectionSender::Connecting => {
                context.stash(OutboundMessage { envelope: self.envelope });
                debug!("message {} to {} is connecting, stash current message and wait", context.myself, addr);
            }
            ConnectionSender::Connected(tx) => {
                if let Some(err) = tx.try_send(self.envelope).err() {
                    match err {
                        TrySendError::Full(_) => {
                            warn!("message {} to {} connection buffer full, current message dropped", context.myself, addr);
                        }
                        TrySendError::Closed(_) => {
                            context.myself.cast(Disconnect { addr }, None);
                            warn!( "message {} to {} connection closed", context.myself, addr );
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
use std::future::Future;
use std::iter::repeat_with;
use std::net::SocketAddr;
use std::pin::Pin;
use std::time::Duration;

use anyhow::anyhow;
use async_trait::async_trait;
use bincode::{Decode, Encode};
use stubborn_io::{ReconnectOptions, StubbornTcpStream};
use tokio::sync::mpsc::error::TrySendError;
use tokio_util::codec::Framed;
use tracing::{debug, error, info, warn};

use actor_core::actor::actor_path::TActorPath;
use actor_core::actor::actor_ref::{ActorRef, ActorRefExt, PROVIDER};
use actor_core::actor::context::{ActorContext, Context};
use actor_core::Message;
use actor_core::message::message_registration::IDPacket;
use actor_derive::EmptyCodec;

use crate::net::codec::PacketCodec;
use crate::net::connection::{Connection, ConnectionTx};
use crate::net::tcp_transport::{ConnectionSender, TransportActor};

#[derive(Debug)]
pub struct RemoteEnvelope {
    pub packet: IDPacket,
    pub sender: Option<ActorRef>,
    pub target: ActorRef,
}

#[derive(Debug, Encode, Decode)]
pub struct RemotePacket {
    pub packet: IDPacket,
    pub sender: Option<String>,
    pub target: String,
}

impl Into<RemotePacket> for RemoteEnvelope {
    fn into(self) -> RemotePacket {
        RemotePacket {
            packet: self.packet,
            sender: self.sender.map(|s| s.path().to_serialization_format()),
            target: self.target.path().to_serialization_format(),
        }
    }
}

#[derive(EmptyCodec)]
pub struct Connect {
    pub addr: SocketAddr,
    pub opts: ReconnectOptions,
}

#[async_trait]
impl Message for Connect {
    type A = TransportActor;

    async fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
        let myself = context.myself().clone();
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

#[async_trait]
impl Message for Connected {
    type A = TransportActor;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        actor.connections.insert(self.addr, ConnectionSender::Connected(self.tx));
        info!("{} connect to {}", context.myself(), self.addr);
        context.unstash_all();
        Ok(())
    }
}

#[derive(EmptyCodec)]
pub struct Disconnect {
    pub addr: SocketAddr,
}

#[async_trait]
impl Message for Disconnect {
    type A = TransportActor;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
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

#[async_trait]
impl Message for SpawnInbound {
    type A = TransportActor;

    async fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
        context.spawn(self.fut);
        Ok(())
    }
}

#[derive(EmptyCodec)]
pub struct InboundMessage {
    pub packet: RemotePacket,
}

#[async_trait]
impl Message for InboundMessage {
    type A = TransportActor;

    async fn handle(self: Box<Self>, _context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let RemotePacket {
            packet,
            sender,
            target,
        } = self.packet;
        let sender = sender.map(|s| actor.resolve_actor_ref(s));
        let target = actor.resolve_actor_ref(target);
        let reg = &actor.registration;
        let message = PROVIDER.sync_scope(actor.provider.clone(), || {
            reg.decode(packet)
        });
        target.tell(message?, sender);
        Ok(())
    }
}

#[derive(Debug, EmptyCodec)]
pub struct OutboundMessage {
    pub name: &'static str,
    pub envelope: RemoteEnvelope,
}

#[async_trait]
impl Message for OutboundMessage {
    type A = TransportActor;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let addr: SocketAddr = self.envelope.target.path().address().addr.map(|a| a.into()).ok_or(anyhow!("socket addr not set"))?;
        let sender = actor.connections.entry(addr).or_insert(ConnectionSender::NotConnected);
        match sender {
            ConnectionSender::NotConnected => {
                let opts = ReconnectOptions::new()
                    .with_exit_if_first_connect_fails(false)
                    .with_retries_generator(|| repeat_with(|| Duration::from_secs(3)));
                context.myself().cast_ns(Connect { addr, opts });
                context.stash(OutboundMessage { name: self.name, envelope: self.envelope });
                debug!("connection to {} not established, stash {} and start connect", addr, self.name);
                *sender = ConnectionSender::Connecting;
            }
            ConnectionSender::Connecting => {
                context.stash(OutboundMessage { name: self.name, envelope: self.envelope });
                debug!("connection to {} is establishing, stash {} and wait it established", addr, self.name);
            }
            ConnectionSender::Connected(tx) => {
                if let Some(err) = tx.try_send(self.envelope).err() {
                    match err {
                        TrySendError::Full(_) => {
                            warn!("message {} to {} connection buffer full, current message dropped", self.name, addr);
                        }
                        TrySendError::Closed(_) => {
                            context.myself().cast(Disconnect { addr }, None);
                            warn!( "message {} to {} connection closed", self.name, addr );
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
use std::any::Any;
use std::future::Future;
use std::iter::repeat_with;
use std::net::SocketAddr;
use std::pin::Pin;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use stubborn_io::{ReconnectOptions, StubbornTcpStream};
use tokio::sync::mpsc::error::TrySendError;
use tokio_util::codec::Framed;
use tracing::{debug, error, info, warn};

use crate::actor::{Actor, CodecMessage, Message};
use crate::actor::context::{ActorContext, Context};
use crate::actor_path::TActorPath;
use crate::actor_ref::{ActorRef, ActorRefExt, SerializedActorRef};
use crate::actor_ref::TActorRef;
use crate::decoder::MessageDecoder;
use crate::message::IDPacket;
use crate::net::codec::PacketCodec;
use crate::net::connection::{Connection, ConnectionTx};
use crate::net::tcp_transport::{ConnectionSender, TransportActor};
use crate::system::ActorSystem;

pub(crate) struct RemoteEnvelope {
    pub(crate) packet: IDPacket,
    pub(crate) sender: Option<ActorRef>,
    pub(crate) target: ActorRef,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct RemotePacket {
    pub(crate) packet: IDPacket,
    pub(crate) sender: Option<SerializedActorRef>,
    pub(crate) target: SerializedActorRef,
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

pub(crate) struct Connect {
    pub(crate) addr: SocketAddr,
    pub(crate) opts: ReconnectOptions,
}


impl CodecMessage for Connect {
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    fn decoder() -> Option<Box<dyn MessageDecoder>> where Self: Sized {
        None
    }

    fn encode(&self) -> Option<anyhow::Result<Vec<u8>>> {
        None
    }
}

impl Message for Connect {
    type T = TransportActor;

    fn handle(self: Box<Self>, context: &mut ActorContext, state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
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

pub(crate) struct Connected {
    pub(crate) addr: SocketAddr,
    pub(crate) tx: ConnectionTx,
}

impl CodecMessage for Connected {
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    fn decoder() -> Option<Box<dyn MessageDecoder>> where Self: Sized {
        None
    }

    fn encode(&self) -> Option<anyhow::Result<Vec<u8>>> {
        None
    }
}

impl Message for Connected {
    type T = TransportActor;

    fn handle(self: Box<Self>, context: &mut ActorContext, state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
        state.connections.insert(self.addr, ConnectionSender::Connected(self.tx));
        info!("{} connect to {}", context.myself, self.addr);
        context.unstash_all();
        Ok(())
    }
}

pub(crate) struct Disconnect {
    pub(crate) addr: SocketAddr,
}

impl CodecMessage for Disconnect {
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    fn decoder() -> Option<Box<dyn MessageDecoder>> where Self: Sized {
        None
    }

    fn encode(&self) -> Option<anyhow::Result<Vec<u8>>> {
        None
    }
}

impl Message for Disconnect {
    type T = TransportActor;

    fn handle(self: Box<Self>, context: &mut ActorContext, state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
        state.connections.remove(&self.addr);
        let myself = context.myself();
        info!("{} disconnect to {}", myself, self.addr);
        Ok(())
    }
}

pub(crate) struct SpawnInbound {
    pub(crate) fut: Pin<Box<dyn Future<Output=()> + Send + 'static>>,
}

impl CodecMessage for SpawnInbound {
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    fn decoder() -> Option<Box<dyn MessageDecoder>> where Self: Sized {
        None
    }

    fn encode(&self) -> Option<anyhow::Result<Vec<u8>>> {
        None
    }
}

impl Message for SpawnInbound {
    type T = TransportActor;

    fn handle(self: Box<Self>, context: &mut ActorContext, state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
        context.spawn(self.fut);
        Ok(())
    }
}

pub(crate) struct InboundMessage {
    pub(crate) packet: RemotePacket,
}

impl CodecMessage for InboundMessage {
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    fn decoder() -> Option<Box<dyn MessageDecoder>> where Self: Sized {
        None
    }

    fn encode(&self) -> Option<anyhow::Result<Vec<u8>>> {
        None
    }
}

impl Message for InboundMessage {
    type T = TransportActor;

    fn handle(self: Box<Self>, context: &mut ActorContext, state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
        let RemotePacket {
            packet,
            sender,
            target,
        } = self.packet;
        let sender = sender.map(|s| state.resolve_actor_ref(context, s).clone());
        let target = state.resolve_actor_ref(context, target);
        let system: ActorSystem = target.system();
        let reg = system.registration();
        let message = reg.decode(packet)?;
        target.tell(message, sender);
        Ok(())
    }
}

pub(crate) struct OutboundMessage {
    pub(crate) envelope: RemoteEnvelope,
}

impl CodecMessage for OutboundMessage {
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    fn decoder() -> Option<Box<dyn MessageDecoder>> where Self: Sized {
        None
    }

    fn encode(&self) -> Option<anyhow::Result<Vec<u8>>> {
        None
    }
}

impl Message for OutboundMessage {
    type T = TransportActor;

    fn handle(self: Box<Self>, context: &mut ActorContext, state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
        let addr: SocketAddr = self.envelope.target.path().address().addr.into();
        let sender = state.connections.entry(addr).or_insert(ConnectionSender::NotConnected);
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
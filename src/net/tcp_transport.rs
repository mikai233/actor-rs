use std::collections::HashMap;
use std::fmt::Debug;
use std::future::Future;
use std::iter::repeat_with;
use std::net::SocketAddr;
use std::num::NonZeroUsize;
use std::pin::Pin;
use std::time::Duration;

use futures::StreamExt;
use lru::LruCache;
use stubborn_io::{ReconnectOptions, StubbornTcpStream};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpListener;
use tokio::sync::mpsc::error::TrySendError;
use tokio::task::JoinHandle;
use tokio_util::codec::Framed;
use tracing::{debug, error, info, warn};

use crate::actor::Actor;
use crate::actor::context::{ActorContext, Context};
use crate::actor_path::TActorPath;
use crate::actor_ref::{ActorRef, ActorRefExt, SerializedActorRef, TActorRef};
use crate::cell::envelope::UserEnvelope;
use crate::ext::decode_bytes;
use crate::message::ActorMessage;
use crate::net::codec::PacketCodec;
use crate::net::connection::{Connection, ConnectionTx};
use crate::net::message::{RemoteEnvelope, RemotePacket};
use crate::provider::{ActorRefFactory, TActorRefProvider};

#[derive(Debug)]
pub(crate) struct TransportActor;

pub(crate) enum TransportMessage {
    Connect(SocketAddr, ReconnectOptions),
    Connected(SocketAddr, ConnectionTx),
    Disconnect(SocketAddr),
    SpawnInbound(Pin<Box<dyn Future<Output=()> + Send + 'static>>),
    InboundMessage(RemotePacket),
    OutboundMessage(RemoteEnvelope),
}

#[derive(Debug)]
enum ConnectionSender {
    NotConnected,
    Connecting,
    Connected(ConnectionTx),
}

impl Actor for TransportActor {
    type S = TcpTransport;
    type A = ();

    fn pre_start(&self, ctx: &mut ActorContext<Self>, _arg: Self::A) -> anyhow::Result<Self::S> {
        let myself = ctx.myself.clone();
        let transport = TcpTransport::new();
        let address = ctx.system.address().clone();
        let addr = address.addr;
        ctx.spawn(async move {
            let tcp_listener = TcpListener::bind(addr).await.unwrap();
            info!("{} start listening", address);
            loop {
                match tcp_listener.accept().await {
                    Ok((stream, peer_addr)) => {
                        let actor = myself.clone();
                        let connection_fut = async move {
                            TcpTransport::accept_inbound_connection(stream, peer_addr, actor).await;
                        };
                        myself.tell_local(
                            TransportMessage::SpawnInbound(Box::pin(connection_fut)),
                            None,
                        );
                    }
                    Err(err) => {
                        warn!("{} accept connection error {:?}", addr, err);
                    }
                }
            }
        });
        Ok(transport)
    }

    fn on_recv(
        &self,
        ctx: &mut ActorContext<Self>,
        state: &mut Self::S,
        envelope: UserEnvelope<Self::M>,
    ) -> anyhow::Result<()> {
        match envelope {
            UserEnvelope::Local(l) => match l {
                TransportMessage::Connect(addr, opts) => {
                    let myself = ctx.myself.clone();
                    ctx.spawn(async move {
                        match StubbornTcpStream::connect_with_options(addr, opts).await {
                            Ok(stream) => {
                                if let Some(e) = stream.set_nodelay(true).err() {
                                    warn!("connect {} set tcp nodelay error {:?}, drop current connection", addr, e);
                                    return;
                                }
                                let framed = Framed::new(stream, PacketCodec);
                                let (connection, tx) = Connection::new(addr, framed, myself.clone());
                                connection.start();
                                myself.tell_local(TransportMessage::Connected(addr, tx), None);
                            }
                            Err(e) => {
                                error!("connect to {} error {:?}, drop current connection", addr, e);
                            }
                        };
                    });
                }
                TransportMessage::Disconnect(addr) => {
                    state.connections.remove(&addr);
                    let myself = ctx.myself();
                    info!("{} disconnect to {}", myself, addr);
                }
                TransportMessage::SpawnInbound(i) => {
                    ctx.spawn(i);
                }
                TransportMessage::InboundMessage(m) => {
                    let RemotePacket {
                        message,
                        sender,
                        target,
                    } = m;
                    let sender = sender.map(|s| state.resolve_actor_ref(ctx, s).clone());
                    let target = state.resolve_actor_ref(ctx, target);
                    target.tell(ActorMessage::Remote(message), sender);
                }
                TransportMessage::Connected(addr, connection) => {
                    state.connections.insert(addr, ConnectionSender::Connected(connection));
                    info!("{} connect to {}", ctx.myself, addr);
                    ctx.unstash_all();
                }
                TransportMessage::OutboundMessage(envelope) => {
                    let addr: SocketAddr = envelope.target.path().address().addr.into();
                    let sender = state.connections.entry(addr).or_insert(ConnectionSender::NotConnected);
                    match sender {
                        ConnectionSender::NotConnected => {
                            let opts = ReconnectOptions::new()
                                .with_exit_if_first_connect_fails(false)
                                .with_retries_generator(|| repeat_with(|| Duration::from_secs(3)));
                            ctx.myself
                                .tell_local(TransportMessage::Connect(addr, opts), None);
                            ctx.stash(UserEnvelope::Local(TransportMessage::OutboundMessage(envelope)));
                            debug!("message {} to {} not connected, stash current message and start connect", ctx.myself, addr);
                            *sender = ConnectionSender::Connecting;
                        }
                        ConnectionSender::Connecting => {
                            ctx.stash(UserEnvelope::Local(TransportMessage::OutboundMessage(envelope)));
                            debug!("message {} to {} is connecting, stash current message and wait", ctx.myself, addr);
                        }
                        ConnectionSender::Connected(tx) => {
                            if let Some(err) = tx.try_send(envelope).err() {
                                match err {
                                    TrySendError::Full(_) => {
                                        warn!("message {} to {} connection buffer full, current message dropped", ctx.myself, addr);
                                    }
                                    TrySendError::Closed(_) => {
                                        ctx.myself.tell_local(TransportMessage::Disconnect(addr), None);
                                        warn!( "message {} to {} connection closed", ctx.myself, addr );
                                    }
                                }
                            }
                        }
                    }
                }
            },
            UserEnvelope::Remote { name, message } => todo!(),
            UserEnvelope::Unknown { name, message } => todo!(),
        }
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct TcpTransport {
    connections: HashMap<SocketAddr, ConnectionSender>,
    actor_cache: LruCache<SerializedActorRef, ActorRef>,
    listener: Option<JoinHandle<()>>,
}

impl TcpTransport {
    pub fn new() -> Self {
        Self {
            connections: HashMap::new(),
            actor_cache: LruCache::new(NonZeroUsize::new(1000).unwrap()),
            listener: None,
        }
    }

    async fn accept_inbound_connection<S>(stream: S, addr: SocketAddr, actor: ActorRef)
        where
            S: Send + AsyncRead + AsyncWrite + Unpin + 'static,
    {
        let mut framed = Framed::new(stream, PacketCodec);
        loop {
            match framed.next().await {
                Some(Ok(packet)) => {
                    match decode_bytes::<RemotePacket>(packet.body.as_slice()) {
                        Ok(envelope) => {
                            actor.tell_local(TransportMessage::InboundMessage(envelope), None);
                        }
                        Err(error) => {
                            warn!("{} deserialize error {:?}", addr, error);
                            break;
                        }
                    }
                }
                Some(Err(error)) => {
                    warn!("{} codec error {:?}", addr, error);
                    break;
                }
                None => {
                    break;
                }
            }
        }
    }

    pub fn resolve_actor_ref(
        &mut self,
        ctx: &mut ActorContext<TransportActor>,
        serialized_ref: SerializedActorRef,
    ) -> &ActorRef {
        self.actor_cache.get_or_insert(serialized_ref.clone(), || {
            ctx.system()
                .provider()
                .resolve_actor_ref(&serialized_ref.path)
        })
    }
}

#[cfg(test)]
mod transport_test {
    use std::time::Duration;

    use serde::{Deserialize, Serialize};
    use tracing::info;

    use crate::actor::Actor;
    use crate::actor::context::{ActorContext, Context};
    use crate::actor_ref::ActorRefExt;
    use crate::cell::envelope::UserEnvelope;
    use crate::ext::decode_bytes;
    use crate::props::Props;
    use crate::provider::ActorRefFactory;
    use crate::provider::TActorRefProvider;
    use crate::system::ActorSystem;

    struct TestActor;

    #[derive(Debug, Serialize, Deserialize)]
    enum TestMessage {
        Ping,
        Pong,
        PingTo(String),
    }

    impl Actor for TestActor {
        type M = TestMessage;
        type S = ();
        type A = ();

        fn pre_start(&self, ctx: &mut ActorContext<Self>, arg: Self::A) -> anyhow::Result<Self::S> {
            info!("{} pre start", ctx.myself);
            Ok(())
        }

        fn on_recv(
            &self,
            ctx: &mut ActorContext<Self>,
            state: &mut Self::S,
            envelope: UserEnvelope<Self::M>,
        ) -> anyhow::Result<()> {
            match envelope {
                UserEnvelope::Local(l) => {
                    match l {
                        TestMessage::PingTo(to) => {
                            let to = ctx.system.provider().resolve_actor_ref(&to);
                            to.tell_remote(&TestMessage::Ping, Some(ctx.myself.clone()))?;
                        }
                        _ => {}
                    }
                }
                UserEnvelope::Remote { name, message } => {
                    let message = decode_bytes::<TestMessage>(message.as_slice())?;
                    info!("{} recv {:?}", ctx.myself, message);
                    match message {
                        TestMessage::Ping => {
                            let myself = ctx.myself.clone();
                            let sender = ctx.sender().unwrap().clone();
                            ctx.spawn(async move {
                                sender.tell_remote(&TestMessage::Pong, Some(myself)).unwrap();
                                tokio::time::sleep(Duration::from_secs(3)).await;
                            });
                        }
                        TestMessage::Pong => {}
                        TestMessage::PingTo(_) => {}
                    }
                    if matches!(message, TestMessage::Ping) {}
                }
                UserEnvelope::Unknown { name, message } => todo!(),
            }
            Ok(())
        }
    }

    #[tokio::test]
    async fn test() -> anyhow::Result<()> {
        let system_a = ActorSystem::new("game".to_string(), "127.0.0.1:12121".parse()?)?;
        let actor_a = system_a.actor_of(TestActor, (), Props::default(), Some("actor_a".to_string()))?;
        let system_a = ActorSystem::new("game".to_string(), "127.0.0.1:12122".parse()?)?;
        let actor_b = system_a.actor_of(TestActor, (), Props::default(), Some("actor_b".to_string()))?;
        loop {
            actor_a.tell_local(TestMessage::PingTo("tcp://game@127.0.0.1:12122/user/actor_b".to_string()), None);
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
}

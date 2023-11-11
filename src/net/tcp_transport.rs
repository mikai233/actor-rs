use std::collections::HashMap;
use std::fmt::Debug;
use std::future::Future;
use std::net::SocketAddr;
use std::num::NonZeroUsize;

use futures::StreamExt;
use lru::LruCache;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tokio_util::codec::Framed;
use tracing::{info, warn};

use crate::actor::Actor;
use crate::actor::context::{ActorContext, Context};
use crate::actor_path::TActorPath;
use crate::actor_ref::{ActorRef, ActorRefExt, SerializedActorRef, TActorRef};
use crate::ext::decode_bytes;
use crate::net::codec::PacketCodec;
use crate::net::connection::ConnectionTx;
use crate::net::message::{InboundMessage, RemotePacket, SpawnInbound};
use crate::provider::{ActorRefFactory, TActorRefProvider};

#[derive(Debug)]
pub(crate) struct TransportActor;

#[derive(Debug)]
pub(crate) enum ConnectionSender {
    NotConnected,
    Connecting,
    Connected(ConnectionTx),
}

impl Actor for TransportActor {
    type S = TcpTransport;
    type A = ();

    fn pre_start(&self, context: &mut ActorContext, _arg: Self::A) -> anyhow::Result<Self::S> {
        let myself = context.myself.clone();
        let transport = TcpTransport::new();
        let address = context.system.address().clone();
        let addr = address.addr;
        context.spawn(async move {
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
                            SpawnInbound { fut: Box::pin(connection_fut) },
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
}

#[derive(Debug)]
pub(crate) struct TcpTransport {
    pub(crate) connections: HashMap<SocketAddr, ConnectionSender>,
    pub(crate) actor_cache: LruCache<SerializedActorRef, ActorRef>,
    pub(crate) listener: Option<JoinHandle<()>>,
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
                        Ok(packet) => {
                            actor.tell_local(InboundMessage { packet }, None);
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

    pub fn resolve_actor_ref(&mut self, ctx: &mut ActorContext, serialized_ref: SerializedActorRef) -> &ActorRef {
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

    use async_trait::async_trait;
    use futures::future::LocalBoxFuture;
    use futures::stream::FuturesUnordered;
    use serde::{Deserialize, Serialize};
    use tracing::info;

    use crate::actor::{Actor, DynamicMessage, Message, MessageDecoder, UserDelegate, RemoteMessage};
    use crate::actor::context::{ActorContext, Context};
    use crate::actor_ref::ActorRefExt;
    use crate::ext::{decode_bytes, encode_bytes};
    use crate::props::Props;
    use crate::provider::ActorRefFactory;
    use crate::provider::TActorRefProvider;
    use crate::system::ActorSystem;
    use crate::user_message_decoder;

    struct TestActor;

    #[derive(Serialize, Deserialize)]
    struct Ping;

    #[async_trait(? Send)]
    impl Message for Ping {
        type T = TestActor;

        async fn handle(self: Box<Self>, context: &mut ActorContext, state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
            let myself = context.myself.clone();
            let sender = context.sender().unwrap().clone();
            context.spawn(async move {
                sender.tell_remote(Pong, Some(myself));
                tokio::time::sleep(Duration::from_secs(3)).await;
            });
            Ok(())
        }
    }

    impl RemoteMessage for Ping {
        fn decoder() -> Box<dyn MessageDecoder> {
            struct D;
            impl MessageDecoder for D {
                fn decode(&self, bytes: &[u8]) -> anyhow::Result<DynamicMessage> {
                    let message: Ping = decode_bytes(bytes)?;
                    let message = UserDelegate::<TestActor>::new(message);
                    Ok(message.into())
                }
            }
            Box::new(D)
        }

        fn encode(&self) -> anyhow::Result<Vec<u8>> {
            encode_bytes(self)
        }
    }

    #[derive(Serialize, Deserialize)]
    struct Pong;

    #[async_trait(? Send)]
    impl Message for Pong {
        type T = TestActor;

        async fn handle(self: Box<Self>, context: &mut ActorContext, state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
            todo!()
        }
    }

    #[async_trait(? Send)]
    impl RemoteMessage for Pong {
        fn decoder() -> Box<dyn MessageDecoder> {
            user_message_decoder!(Pong, TestActor)
        }

        fn encode(&self) -> anyhow::Result<Vec<u8>> {
            encode_bytes(self)
        }
    }

    struct PingTo {
        to: String,
    }

    #[async_trait(? Send)]
    impl Message for PingTo {
        type T = TestActor;

        async fn handle(self: Box<Self>, context: &mut ActorContext, state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
            let to = context.system.provider().resolve_actor_ref(&self.to);
            to.tell_remote(Ping, Some(context.myself.clone()));
            Ok(())
        }
    }

    impl Actor for TestActor {
        type S = ();
        type A = ();

        fn pre_start(&self, ctx: &mut ActorContext, arg: Self::A) -> anyhow::Result<Self::S> {
            info!("{} pre start", ctx.myself);
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
            actor_a.tell_local(PingTo { to: "tcp://game@127.0.0.1:12122/user/actor_b".to_string() }, None);
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
}

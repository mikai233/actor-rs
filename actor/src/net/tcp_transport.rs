use std::collections::HashMap;
use std::fmt::Debug;
use std::net::SocketAddr;

use futures::StreamExt;
use moka::sync::Cache;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tokio_util::codec::Framed;
use tracing::{info, warn};

use crate::Actor;
use crate::actor_ref::{ActorRef, ActorRefExt, SerializedActorRef};
use crate::context::ActorContext;
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
                        myself.cast(
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
    pub(crate) actor_ref_cache: Cache<SerializedActorRef, ActorRef>,
    pub(crate) listener: Option<JoinHandle<()>>,
}

impl TcpTransport {
    pub fn new() -> Self {
        Self {
            connections: HashMap::new(),
            actor_ref_cache: Cache::new(1000),
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
                            actor.cast(InboundMessage { packet }, None);
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

    pub fn resolve_actor_ref(&mut self, context: &mut ActorContext, serialized_ref: SerializedActorRef) -> ActorRef {
        self.actor_ref_cache.get_with_by_ref(&serialized_ref, || {
            context.system()
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

    use actor_derive::{EmptyCodec, MessageCodec};

    use crate::{Actor, Message};
    use crate::actor_ref::ActorRefExt;
    use crate::context::{ActorContext, Context};
    use crate::message::MessageRegistration;
    use crate::props::Props;
    use crate::provider::ActorRefFactory;
    use crate::provider::TActorRefProvider;
    use crate::system::ActorSystem;

    struct PingPongActor;

    #[derive(Serialize, Deserialize, MessageCodec)]
    #[actor(PingPongActor)]
    struct Ping;

    impl Message for Ping {
        type T = PingPongActor;

        fn handle(self: Box<Self>, context: &mut ActorContext, _state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
            let myself = context.myself.clone();
            let sender = context.sender().unwrap().clone();
            context.spawn(async move {
                sender.cast(Pong, Some(myself));
                tokio::time::sleep(Duration::from_secs(1)).await;
            });
            Ok(())
        }
    }

    #[derive(Serialize, Deserialize, MessageCodec)]
    #[actor(PingPongActor)]
    struct Pong;

    impl Message for Pong {
        type T = PingPongActor;

        fn handle(self: Box<Self>, context: &mut ActorContext, _state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
            info!("{} pong", context.myself());
            Ok(())
        }
    }

    #[derive(EmptyCodec)]
    struct PingTo {
        to: String,
    }

    impl Message for PingTo {
        type T = PingPongActor;

        fn handle(self: Box<Self>, context: &mut ActorContext, _state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
            let to = context.system.provider().resolve_actor_ref(&self.to);
            to.cast(Ping, Some(context.myself.clone()));
            Ok(())
        }
    }

    impl Actor for PingPongActor {
        type S = ();
        type A = ();

        fn pre_start(&self, context: &mut ActorContext, _arg: Self::A) -> anyhow::Result<Self::S> {
            info!("{} pre start", context.myself);
            Ok(())
        }
    }

    fn new_message_reg() -> MessageRegistration {
        let mut reg = MessageRegistration::new();
        reg.register::<Ping>();
        reg.register::<Pong>();
        reg
    }

    #[tokio::test]
    async fn test() -> anyhow::Result<()> {
        let system_a = ActorSystem::new("game".to_string(), "127.0.0.1:12121".parse()?, new_message_reg())?;
        let actor_a = system_a.actor_of(PingPongActor, (), Props::default(), Some("actor_a".to_string()))?;
        let system_a = ActorSystem::new("game".to_string(), "127.0.0.1:12122".parse()?, new_message_reg())?;
        let _ = system_a.actor_of(PingPongActor, (), Props::default(), Some("actor_b".to_string()))?;
        loop {
            actor_a.cast(PingTo { to: "tcp://game@127.0.0.1:12122/user/actor_b".to_string() }, None);
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
}

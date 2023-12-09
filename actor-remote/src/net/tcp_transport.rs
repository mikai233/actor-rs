use std::collections::HashMap;
use std::fmt::Debug;
use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::anyhow;
use async_trait::async_trait;
use futures::StreamExt;
use moka::sync::Cache;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpListener;
use tokio_util::codec::Framed;
use tracing::{info, warn};

use actor_core::Actor;
use actor_core::actor::actor_ref::{ActorRef, ActorRefExt};
use actor_core::actor::actor_ref_factory::ActorRefFactory;
use actor_core::actor::actor_ref_provider::ActorRefProvider;
use actor_core::actor::actor_system::ActorSystem;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::serialized_ref::SerializedActorRef;
use actor_core::ext::decode_bytes;

use crate::message_registration::MessageRegistration;
use crate::net::codec::PacketCodec;
use crate::net::connection::ConnectionTx;
use crate::net::message::{InboundMessage, RemotePacket, SpawnInbound};
use crate::remote_provider::RemoteActorRefProvider;

#[derive(Debug)]
pub struct TransportActor {
    pub connections: HashMap<SocketAddr, ConnectionSender>,
    pub actor_ref_cache: Cache<SerializedActorRef, ActorRef>,
    pub provider: Arc<ActorRefProvider>,
    pub registration: MessageRegistration,
}

#[derive(Debug)]
pub enum ConnectionSender {
    NotConnected,
    Connecting,
    Connected(ConnectionTx),
}

#[async_trait]
impl Actor for TransportActor {
    async fn pre_start(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        let myself = context.myself().clone();
        let address = context.system().address().clone();
        let addr = address.addr.ok_or(anyhow!("socket addr not set"))?;
        context.spawn(async move {
            let tcp_listener = TcpListener::bind(addr).await.unwrap();
            info!("{} start listening", address);
            loop {
                match tcp_listener.accept().await {
                    Ok((stream, peer_addr)) => {
                        let actor = myself.clone();
                        let connection_fut = async move {
                            TransportActor::accept_inbound_connection(stream, peer_addr, actor).await;
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
        Ok(())
    }
}


impl TransportActor {
    pub fn new(system: ActorSystem) -> Self {
        let provider = system.provider();
        let registration = (&***provider)
            .as_any_ref()
            .downcast_ref::<RemoteActorRefProvider>()
            .unwrap()
            .registration
            .clone();
        Self {
            connections: HashMap::new(),
            actor_ref_cache: Cache::new(1000),
            provider,
            registration: (*registration).clone(),
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

    pub fn resolve_actor_ref(&mut self, serialized_ref: SerializedActorRef) -> ActorRef {
        self.actor_ref_cache.get_with_by_ref(&serialized_ref, || {
            self.provider.resolve_actor_ref(&serialized_ref.path)
        })
    }
}

#[cfg(test)]
mod test {
    use std::net::SocketAddrV4;
    use std::time::Duration;

    use async_trait::async_trait;
    use serde::{Deserialize, Serialize};
    use tracing::info;

    use actor_core::{Actor, Message};
    use actor_core::actor::actor_ref::ActorRefExt;
    use actor_core::actor::actor_ref_factory::ActorRefFactory;
    use actor_core::actor::actor_system::ActorSystem;
    use actor_core::actor::config::actor_system_config::ActorSystemConfig;
    use actor_core::actor::context::{ActorContext, Context};
    use actor_core::actor::props::Props;
    use actor_derive::{EmptyCodec, MessageCodec};

    use crate::message_registration::MessageRegistration;
    use crate::remote_provider::RemoteActorRefProvider;

    struct PingPongActor;

    #[derive(Serialize, Deserialize, MessageCodec)]
    #[actor(PingPongActor)]
    struct Ping;

    impl Message for Ping {
        type A = PingPongActor;

        fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
            let myself = context.myself().clone();
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
        type A = PingPongActor;

        fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
            info!("{} pong", context.myself());
            Ok(())
        }
    }

    #[derive(EmptyCodec)]
    struct PingTo {
        to: String,
    }

    impl Message for PingTo {
        type A = PingPongActor;

        fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
            let to = context.system().provider().resolve_actor_ref(&self.to);
            to.cast(Ping, Some(context.myself().clone()));
            Ok(())
        }
    }

    #[async_trait]
    impl Actor for PingPongActor {
        async fn pre_start(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
            info!("{} pre start", context.myself());
            Ok(())
        }
    }

    fn build_config(addr: SocketAddrV4) -> ActorSystemConfig {
        let mut config = ActorSystemConfig::default();
        config.with_provider(move |system| {
            let mut registration = MessageRegistration::new();
            registration.register::<Ping>();
            registration.register::<Pong>();
            RemoteActorRefProvider::new(system, registration, addr).map(|(r, d)| (r.into(), d))
        });
        config
    }

    #[tokio::test]
    async fn test() -> anyhow::Result<()> {
        let system_a = ActorSystem::create("game", build_config("127.0.0.1:12121".parse()?))?;
        let props = Props::create(|_| PingPongActor);
        let actor_a = system_a.spawn_actor(props.clone(), "actor_a")?;
        let system_b = ActorSystem::create("game", build_config("127.0.0.1:12122".parse()?))?;
        let _ = system_b.spawn_actor(props.clone(), "actor_b")?;
        loop {
            actor_a.cast(PingTo { to: "tcp://game@127.0.0.1:12122/user/actor_b".to_string() }, None);
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
}

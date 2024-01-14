use std::collections::{HashMap, VecDeque};
use std::fmt::Debug;
use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::anyhow;
use async_trait::async_trait;
use futures::StreamExt;
use quick_cache::unsync::Cache;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpListener;
use tokio::task::AbortHandle;
use tokio_util::codec::Framed;
use tracing::{info, warn};

use actor_core::{Actor, DynMessage};
use actor_core::actor::actor_ref::{ActorRef, ActorRefExt};
use actor_core::actor::actor_ref_factory::ActorRefFactory;
use actor_core::actor::actor_ref_provider::ActorRefProvider;
use actor_core::actor::actor_system::ActorSystem;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::ext::decode_bytes;
use actor_core::message::message_registration::MessageRegistration;

use crate::net::codec::PacketCodec;
use crate::net::connection::ConnectionTx;
use crate::net::message::{InboundMessage, OutboundMessage, RemotePacket, SpawnInbound};

pub const BUFFER_SIZE: usize = 10000;

#[derive(Debug)]
pub struct TransportActor {
    pub connections: HashMap<SocketAddr, ConnectionStatus>,
    pub actor_ref_cache: Cache<String, ActorRef>,
    pub provider: Arc<ActorRefProvider>,
    pub registration: MessageRegistration,
    pub buffer: HashMap<SocketAddr, VecDeque<(DynMessage, Option<ActorRef>)>>,
}

#[derive(Debug)]
pub enum ConnectionStatus {
    NotConnected,
    PrepareForConnect,
    Connecting(AbortHandle),
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
                        myself.cast_ns(SpawnInbound { fut: Box::pin(connection_fut) });
                    }
                    Err(error) => {
                        warn!("{} accept connection error {:?}", addr, error);
                    }
                }
            }
        });
        Ok(())
    }
}


impl TransportActor {
    pub fn new(system: ActorSystem) -> Self {
        let provider = system.provider_full();
        let registration = provider
            .registration()
            .map(|r| (**r).clone())
            .expect("message registration not found");
        Self {
            connections: HashMap::new(),
            actor_ref_cache: Cache::new(1000),
            provider,
            registration,
            buffer: HashMap::new(),
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
                            actor.cast_ns(InboundMessage { packet });
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

    pub fn resolve_actor_ref(&mut self, path: String) -> ActorRef {
        match self.actor_ref_cache.get(&path) {
            None => {
                let actor_ref = self.provider.resolve_actor_ref(&path);
                self.actor_ref_cache.insert(path, actor_ref.clone());
                actor_ref
            }
            Some(actor_ref) => {
                actor_ref.clone()
            }
        }
    }

    pub fn stash_message(buffer: &mut HashMap<SocketAddr, VecDeque<(DynMessage, Option<ActorRef>)>>, addr: SocketAddr, msg: OutboundMessage, sender: Option<ActorRef>) {
        let buffer = buffer.entry(addr).or_insert(VecDeque::with_capacity(10));
        buffer.push_back((DynMessage::user(msg), sender));
        if buffer.len() >= BUFFER_SIZE {
            if let Some((message, _)) = buffer.pop_front() {
                warn!("stash buffer is going to large than {}, drop the oldest message {}", BUFFER_SIZE, message.name());
            }
        }
    }
}

#[cfg(test)]
mod test {
    use std::net::SocketAddrV4;
    use std::time::{Duration, SystemTime};

    use async_trait::async_trait;
    use bincode::{Decode, Encode};
    use tracing::info;

    use actor_core::{Actor, EmptyTestActor, Message};
    use actor_core::actor::actor_ref::ActorRefExt;
    use actor_core::actor::actor_ref_factory::ActorRefFactory;
    use actor_core::actor::actor_system::ActorSystem;
    use actor_core::actor::config::actor_setting::ActorSetting;
    use actor_core::actor::context::{ActorContext, Context};
    use actor_core::actor::deferred_ref::Patterns;
    use actor_core::actor::props::Props;
    use actor_core::message::message_registration::MessageRegistration;
    use actor_derive::{EmptyCodec, MessageCodec, OrphanCodec};

    use crate::remote_provider::RemoteActorRefProvider;
    use crate::remote_setting::RemoteSetting;

    struct PingPongActor;

    #[derive(Encode, Decode, MessageCodec)]
    struct Ping;

    #[async_trait]
    impl Message for Ping {
        type A = PingPongActor;

        async fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
            let myself = context.myself().clone();
            let sender = context.sender().unwrap().clone();
            context.spawn(async move {
                sender.cast(Pong, Some(myself));
                tokio::time::sleep(Duration::from_secs(1)).await;
            });
            Ok(())
        }
    }

    #[derive(Encode, Decode, MessageCodec)]
    struct Pong;

    #[async_trait]
    impl Message for Pong {
        type A = PingPongActor;

        async fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
            info!("{} pong", context.myself());
            Ok(())
        }
    }

    #[derive(EmptyCodec)]
    struct PingTo {
        to: String,
    }

    #[async_trait]
    impl Message for PingTo {
        type A = PingPongActor;

        async fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
            let to = context.system().provider().resolve_actor_ref(&self.to);
            to.cast(Ping, Some(context.myself().clone()));
            Ok(())
        }
    }

    #[async_trait]
    impl Actor for PingPongActor {
        async fn pre_start(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
            info!("{} started", context.myself());
            Ok(())
        }
    }

    fn build_setting(addr: SocketAddrV4) -> ActorSetting {
        let mut config = ActorSetting::default();
        config.with_provider(move |system| {
            let mut reg = MessageRegistration::new();
            reg.register_user::<Ping>();
            reg.register_user::<Pong>();
            reg.register_user::<MessageToAsk>();
            reg.register_user::<MessageToAns>();
            let setting = RemoteSetting::builder()
                .system(system.clone())
                .addr(addr)
                .reg(reg)
                .build();
            RemoteActorRefProvider::new(setting).map(|(r, d)| (r.into(), d))
        });
        config
    }

    #[tokio::test]
    async fn test() -> anyhow::Result<()> {
        let system_a = ActorSystem::create("game", build_setting("127.0.0.1:12121".parse()?))?;
        let props = Props::create(|_| PingPongActor);
        let actor_a = system_a.spawn_actor(props.clone(), "actor_a")?;
        let system_b = ActorSystem::create("game", build_setting("127.0.0.1:12122".parse()?))?;
        let _ = system_b.spawn_actor(props.clone(), "actor_b")?;
        loop {
            actor_a.cast(PingTo { to: "tcp://game@127.0.0.1:12122/user/actor_b".to_string() }, None);
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }

    #[derive(Encode, Decode, MessageCodec)]
    struct MessageToAsk;

    #[async_trait]
    impl Message for MessageToAsk {
        type A = EmptyTestActor;

        async fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
            context.sender().unwrap().resp(MessageToAns {
                content: "hello world".to_string(),
            });
            anyhow::Ok(())
        }
    }

    #[derive(Encode, Decode, OrphanCodec)]
    struct MessageToAns {
        content: String,
    }

    #[tokio::test]
    async fn test_remote_ask() -> anyhow::Result<()> {
        let system1 = ActorSystem::create("mikai233", build_setting("127.0.0.1:12121".parse()?))?;
        let system2 = ActorSystem::create("mikai233", build_setting("127.0.0.1:12123".parse()?))?;
        let actor_a = system1.spawn_anonymous_actor(Props::create(|_| EmptyTestActor))?;
        let actor_a = system2.provider().resolve_actor_ref_of_path(actor_a.path());
        let _: MessageToAns = Patterns::ask(&actor_a, MessageToAsk, Duration::from_secs(3)).await?;
        let start = SystemTime::now();
        let range = 0..10000;
        for _ in range {
            let _: MessageToAns = Patterns::ask(&actor_a, MessageToAsk, Duration::from_secs(3)).await?;
        }
        let end = SystemTime::now();
        let cost = end.duration_since(start)?;
        info!("cost {:?}", cost);
        let qps = 1000000.0 / cost.as_millis() as f64;
        info!("{} per/millis",qps);
        Ok(())
    }
}

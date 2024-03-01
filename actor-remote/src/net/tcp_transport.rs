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
use actor_core::actor::coordinated_shutdown::{ActorSystemStartFailedReason, CoordinatedShutdown};
use actor_core::ext::decode_bytes;
use actor_core::message::message_registration::MessageRegistration;

use crate::config::transport::TcpTransport;
use crate::net::codec::PacketCodec;
use crate::net::connection::ConnectionTx;
use crate::net::message::{InboundMessage, OutboundMessage, RemotePacket, SpawnInbound};

pub const ACTOR_REF_CACHE: usize = 10000;

#[derive(Debug)]
pub struct TcpTransportActor {
    pub(crate) transport: TcpTransport,
    pub(crate) connections: HashMap<SocketAddr, ConnectionStatus>,
    pub(crate) actor_ref_cache: Cache<String, ActorRef>,
    pub(crate) provider: Arc<ActorRefProvider>,
    pub(crate) registration: MessageRegistration,
    pub(crate) buffer: HashMap<SocketAddr, VecDeque<(DynMessage, Option<ActorRef>)>>,
}

#[derive(Debug)]
pub enum ConnectionStatus {
    NotConnected,
    PrepareForConnect,
    Connecting(AbortHandle),
    Connected(ConnectionTx),
}

#[async_trait]
impl Actor for TcpTransportActor {
    async fn started(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        let myself = context.myself().clone();
        let addr = self.transport.addr;
        context.spawn_fut(async move {
            info!("start bind tcp addr {}", addr);
            match TcpListener::bind(addr).await {
                Ok(tcp_listener) => {
                    loop {
                        match tcp_listener.accept().await {
                            Ok((stream, peer_addr)) => {
                                let actor = myself.clone();
                                let connection_fut = async move {
                                    TcpTransportActor::accept_inbound_connection(stream, peer_addr, actor).await;
                                };
                                myself.cast_ns(SpawnInbound { fut: Box::pin(connection_fut) });
                            }
                            Err(error) => {
                                warn!("{} accept connection error {:?}", addr, error);
                            }
                        }
                    }
                }
                Err(error) => {
                    //这里不能用actor的上下文去执行异步任务，因为停止系统会导致actor销毁，与之关联的异步任务也会销毁，后面的任务无法执行
                    let fut = {
                        let error = anyhow!("bind tcp addr {} failed {:?}", addr, error);
                        CoordinatedShutdown::get_mut(myself.system()).run_with_result(ActorSystemStartFailedReason, Err(error))
                    };
                    tokio::spawn(async move {
                        fut.await;
                    });
                }
            }
        });
        Ok(())
    }
}


impl TcpTransportActor {
    pub fn new(system: ActorSystem, transport: TcpTransport) -> Self {
        let provider = system.provider_full();
        let registration = provider
            .registration()
            .map(|r| (**r).clone())
            .expect("message registration not found");
        Self {
            transport,
            connections: HashMap::new(),
            actor_ref_cache: Cache::new(ACTOR_REF_CACHE),
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

    pub fn stash_message(buffer: &mut HashMap<SocketAddr, VecDeque<(DynMessage, Option<ActorRef>)>>, transport: &TcpTransport, addr: SocketAddr, msg: OutboundMessage, sender: Option<ActorRef>) {
        let buffer = buffer.entry(addr).or_insert(VecDeque::with_capacity(10));
        buffer.push_back((DynMessage::user(msg), sender));
        if let Some(buffer_len) = transport.buffer {
            if buffer.len() > buffer_len {
                if let Some((message, _)) = buffer.pop_front() {
                    warn!("stash buffer is going to large than {}, drop the oldest message {}", buffer_len, message.name());
                }
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
    use actor_core::actor::context::{ActorContext, Context};
    use actor_core::actor::deferred_ref::Patterns;
    use actor_core::actor::props::Props;
    use actor_core::config::actor_setting::ActorSetting;
    use actor_derive::{EmptyCodec, MessageCodec, OrphanCodec};

    use crate::config::RemoteConfig;
    use crate::config::transport::Transport;
    use crate::remote_provider::RemoteActorRefProvider;

    struct PingPongActor;

    #[derive(Encode, Decode, MessageCodec)]
    struct Ping;

    #[async_trait]
    impl Message for Ping {
        type A = PingPongActor;

        async fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
            let myself = context.myself().clone();
            let sender = context.sender().unwrap().clone();
            context.spawn_fut(async move {
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
        async fn started(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
            info!("{} started", context.myself());
            Ok(())
        }
    }

    fn build_setting(addr: SocketAddrV4) -> ActorSetting {
        ActorSetting::builder()
            .provider_fn(move |system| {
                RemoteActorRefProvider::builder()
                    .config(RemoteConfig { transport: Transport::tcp(addr, None) })
                    .register::<Ping>()
                    .register::<Pong>()
                    .register::<MessageToAsk>()
                    .register::<MessageToAns>()
                    .build(system.clone())
            })
            .build()
    }

    #[tokio::test]
    async fn test() -> anyhow::Result<()> {
        let system_a = ActorSystem::create("game", build_setting("127.0.0.1:12121".parse()?))?;
        let props = Props::new_with_ctx(|_| PingPongActor);
        let actor_a = system_a.spawn(props.clone(), "actor_a")?;
        let system_b = ActorSystem::create("game", build_setting("127.0.0.1:12122".parse()?))?;
        let _ = system_b.spawn(props.clone(), "actor_b")?;
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
        let actor_a = system1.spawn_anonymous(Props::new_with_ctx(|_| EmptyTestActor))?;
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

use std::net::SocketAddr;
use std::sync::Arc;

use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::message::{DynMessage, Message};
use ahash::{HashMap, HashMapExt};
use anyhow::anyhow;
use futures::StreamExt;
use quick_cache::unsync::Cache;
use tokio::io::AsyncRead;
use tokio::net::TcpListener;
use tokio_util::codec::FramedRead;
use tracing::{debug, info, warn};

use actor_core::actor::actor_system::ActorSystem;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::coordinated_shutdown::ActorSystemStartFailedReason;
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::actor_ref::{ActorRef, ActorRefExt};
use actor_core::message::message_buffer::MessageBufferMap;
use actor_core::provider::ActorRefProvider;

use crate::artery::codec::PacketCodec;
use crate::artery::connection_status::ConnectionStatus;
use crate::artery::inbound_message::InboundMessage;
use crate::artery::outbound_message::OutboundMessage;
use crate::artery::remote_packet::RemotePacket;
use crate::artery::spawn_inbound::SpawnInbound;
use crate::artery::transport_buffer_envelop::ArteryBufferEnvelope;
use crate::codec::MessageCodecRegistry;
use crate::config::advanced::Advanced;
use crate::config::artery::Transport;
use crate::config::buffer_type::BufferType;
use crate::remote_provider::RemoteActorRefProvider;

pub mod codec;
pub mod connect_tcp;
pub mod connect_tcp_failed;
pub mod connected;
pub mod connection;
pub mod connection_status;
pub mod disconnect;
pub mod disconnected;
pub mod inbound_message;
pub mod outbound_message;
pub mod remote_packet;
pub mod spawn_inbound;
pub mod transport_buffer_envelop;

pub const ACTOR_REF_CACHE: usize = 10000;

#[derive(Debug)]
pub struct ArteryActor {
    transport: Transport,
    socket_addr: SocketAddr,
    advanced: Advanced,
    connections: HashMap<SocketAddr, ConnectionStatus>,
    actor_ref_cache: Cache<String, ActorRef>,
    provider: ActorRefProvider,
    registration: Arc<dyn MessageCodecRegistry>,
    message_buffer: MessageBufferMap<SocketAddr, ArteryBufferEnvelope>,
}

impl Actor for ArteryActor {
    type Context = Context;

    fn started(&mut self, ctx: &mut Self::Context) -> anyhow::Result<()> {
        Self::spawn_tcp_listener(ctx, self.socket_addr)?;
        Ok(())
    }

    fn receive(&self) -> Receive<Self> {
        Receive::new()
    }
}

impl ArteryActor {
    pub(crate) fn new(
        system: ActorSystem,
        transport: Transport,
        socket_addr: SocketAddr,
        advanced: Advanced,
    ) -> anyhow::Result<Self> {
        let provider = system.provider_full();
        let remote_provider = provider
            .downcast_ref::<RemoteActorRefProvider>()
            .ok_or(anyhow!("RemoteActorRefProvider not found"))?;
        let registration = (*remote_provider.registry).clone();
        let actor = Self {
            transport,
            socket_addr,
            advanced,
            connections: HashMap::new(),
            actor_ref_cache: Cache::new(ACTOR_REF_CACHE),
            provider,
            registration,
            message_buffer: Default::default(),
        };
        Ok(actor)
    }

    async fn accept_inbound_connection<S>(stream: S, addr: SocketAddr, actor: ActorRef)
    where
        S: Send + AsyncRead + Unpin + 'static,
    {
        let mut framed = FramedRead::new(stream, PacketCodec);
        loop {
            match framed.next().await {
                Some(Ok(packet)) => match decode_bytes::<RemotePacket>(packet.body.as_slice()) {
                    Ok(packet) => {
                        actor.cast_ns(InboundMessage(packet));
                    }
                    Err(error) => {
                        warn!("{} deserialize error {:?}", addr, error);
                        break;
                    }
                },
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

    fn resolve_actor_ref(&mut self, path: String) -> ActorRef {
        match self.actor_ref_cache.get(&path) {
            None => {
                let actor_ref = self.provider.resolve_actor_ref(&path);
                self.actor_ref_cache.insert(path, actor_ref.clone());
                actor_ref
            }
            Some(actor_ref) => actor_ref.clone(),
        }
    }

    fn buffer_message<M>(
        message_buffer: &mut MessageBufferMap<SocketAddr, ArteryBufferEnvelope>,
        target_addr: SocketAddr,
        message: M,
        sender: Option<ActorRef>,
        buffer_type: BufferType,
    ) where
        M: Message,
    {
        let envelope = ArteryBufferEnvelope {
            message: Box::new(message),
            sender,
        };
        match buffer_type {
            BufferType::Drop => {
                debug!(
                    "no buffer configured, drop buffer message {}",
                    envelope.message.signature()
                );
            }
            BufferType::Bound(size) => {
                message_buffer.push(target_addr, envelope);
                if message_buffer.total_size() > size {
                    for envelope in message_buffer.drop_first_n(&target_addr, 1) {
                        let signature = envelope.message.signature();
                        warn!("stash buffer is going to large than {size}, drop the oldest message `{signature}`");
                    }
                }
            }
            BufferType::Unbound => {
                message_buffer.push(target_addr, envelope);
            }
        }
    }

    fn spawn_tcp_listener(
        ctx: &mut <ArteryActor as Actor>::Context,
        addr: SocketAddr,
    ) -> anyhow::Result<()> {
        let context = ctx.context();
        let myself = context.myself().clone();
        let system = context.system().clone();
        ctx.spawn_fut(format!("tcp_listener_{}", addr), async move {
            info!("start bind tcp addr {}", addr);
            match TcpListener::bind(addr).await {
                Ok(tcp_listener) => loop {
                    match tcp_listener.accept().await {
                        Ok((stream, peer_addr)) => {
                            let actor = myself.clone();
                            let connection_fut = async move {
                                ArteryActor::accept_inbound_connection(stream, peer_addr, actor)
                                    .await;
                            };
                            myself.cast_ns(SpawnInbound {
                                peer_addr,
                                fut: Box::pin(connection_fut),
                            });
                        }
                        Err(error) => {
                            warn!("{} accept connection error {:?}", addr, error);
                        }
                    }
                },
                Err(error) => {
                    let fut = system.run_coordinated_shutdown(ActorSystemStartFailedReason(
                        anyhow::Error::from(error),
                    ));
                    tokio::spawn(fut);
                }
            }
        })?;
        Ok(())
    }

    fn is_connecting_or_connected(&self, addr: &SocketAddr) -> bool {
        if let Some(status) = self.connections.get(&addr) {
            match status {
                ConnectionStatus::Connecting(_) | ConnectionStatus::Connected(_) => {
                    return true;
                }
                _ => {}
            }
        }
        return false;
    }
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddrV4;
    use std::time::{Duration, SystemTime};

    use actor_core::actor::behavior::Behavior;
    use actor_core::actor::receive::Receive;
    use actor_core::actor::Actor;
    use actor_core::message::handler::MessageHandler;
    use anyhow::anyhow;
    use serde::{Deserialize, Serialize};
    use tracing::info;

    use actor_core::actor::actor_system::ActorSystem;
    use actor_core::actor::context::{ActorContext, Context};
    use actor_core::actor::props::Props;
    use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
    use actor_core::actor_ref::{ActorRef, ActorRefExt};
    use actor_core::pattern::patterns::Patterns;
    use actor_core::{Message, MessageCodec};

    use crate::config::buffer_type::BufferType;
    use crate::config::settings::Remote;
    use crate::remote_provider::RemoteActorRefProvider;

    #[derive(Debug)]
    struct PingPongActor;

    impl Actor for PingPongActor {
        type Context = Context;

        fn receive(&self) -> Receive<Self> {
            Receive::new()
        }
    }

    #[derive(Debug, Serialize, Deserialize, Message, MessageCodec, derive_more::Display)]
    #[display("Ping")]
    struct Ping;

    impl MessageHandler<PingPongActor> for Ping {
        fn handle(
            _: &mut PingPongActor,
            ctx: &mut <PingPongActor as actor_core::actor::Actor>::Context,
            _: Self,
            sender: Option<ActorRef>,
            _: &Receive<PingPongActor>,
        ) -> anyhow::Result<Behavior<PingPongActor>> {
            let context = ctx.context_mut();
            let myself = context.myself().clone();
            context.spawn_fut("pong", async move {
                if let Some(sender) = sender {
                    sender.cast(Pong, Some(myself));
                }
                tokio::time::sleep(Duration::from_secs(1)).await;
            })?;
            Ok(Behavior::same())
        }
    }

    #[derive(Debug, Serialize, Deserialize, Message, MessageCodec, derive_more::Display)]
    #[display("Pong")]
    struct Pong;

    impl MessageHandler<PingPongActor> for Pong {
        fn handle(
            _: &mut PingPongActor,
            ctx: &mut <PingPongActor as Actor>::Context,
            _: Self,
            _: Option<ActorRef>,
            _: &Receive<PingPongActor>,
        ) -> anyhow::Result<Behavior<PingPongActor>> {
            let context = ctx.context();
            info!("{} pong", context.myself());
            Ok(Behavior::same())
        }
    }

    #[derive(Debug, Serialize, Deserialize, Message, MessageCodec, derive_more::Display)]
    #[display("PingTo")]
    struct PingTo {
        to: String,
    }

    impl MessageHandler<PingPongActor> for PingTo {
        fn handle(
            _: &mut PingPongActor,
            ctx: &mut <PingPongActor as Actor>::Context,
            message: Self,
            _: Option<ActorRef>,
            _: &Receive<PingPongActor>,
        ) -> anyhow::Result<Behavior<PingPongActor>> {
            let context = ctx.context();
            let to = context.system().provider().resolve_actor_ref(&message.to);
            to.cast(Ping, Some(context.myself().clone()));
            Ok(Behavior::same())
        }
    }

    fn build_setting(addr: SocketAddrV4) -> anyhow::Result<ActorSetting> {
        let mut remote_setting = Remote {
            config: Remote {
                transport: Transport::tcp(addr, BufferType::default()),
            },
            reg: Default::default(),
        };
        remote_setting.reg.register_user::<Ping>();
        remote_setting.reg.register_user::<Pong>();
        remote_setting.reg.register_user::<MessageToAsk>();
        remote_setting.reg.register_user::<MessageToAns>();
        ActorSetting::new_with_default_config(RemoteActorRefProvider::builder(remote_setting))
    }

    #[tokio::test]
    async fn test() -> anyhow::Result<()> {
        let system_a = ActorSystem::new("game", build_setting("127.0.0.1:12121".parse()?)?)?;
        let actor_a = system_a.spawn(Props::new(|| Ok(PingPongActor)), "actor_a")?;
        let system_b = ActorSystem::new("game", build_setting("127.0.0.1:12122".parse()?)?)?;
        let _ = system_b.spawn(Props::new(|| Ok(PingPongActor)), "actor_b")?;
        loop {
            actor_a.cast(
                PingTo {
                    to: "tcp://game@127.0.0.1:12122/user/actor_b".to_string(),
                },
                None,
            );
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }

    #[derive(Debug)]
    struct EmptyTestActor;

    impl Actor for EmptyTestActor {
        type Context = Context;

        fn receive(&self) -> Receive<Self> {
            Receive::new().handle::<MessageToAsk>()
        }
    }

    #[derive(Debug, Serialize, Deserialize, Message, MessageCodec, derive_more::Display)]
    #[display("MessageToAsk")]
    struct MessageToAsk;

    impl MessageHandler<EmptyTestActor> for MessageToAsk {
        fn handle(
            _: &mut EmptyTestActor,
            _: &mut <EmptyTestActor as Actor>::Context,
            _: Self,
            sender: Option<ActorRef>,
            _: &Receive<EmptyTestActor>,
        ) -> anyhow::Result<Behavior<EmptyTestActor>> {
            sender
                .ok_or(anyhow!("sender not found"))?
                .cast_ns(MessageToAns::new("hello world".to_string()));
            Ok(Behavior::same())
        }
    }

    #[derive(
        Debug,
        Serialize,
        Deserialize,
        Message,
        MessageCodec,
        derive_more::Display,
        derive_more::Constructor,
    )]
    #[display("MessageToAns")]
    struct MessageToAns {
        content: String,
    }

    #[tokio::test]
    async fn test_remote_ask() -> anyhow::Result<()> {
        let system1 = ActorSystem::new("mikai233", build_setting("127.0.0.1:12121".parse()?)?)?;
        let system2 = ActorSystem::new("mikai233", build_setting("127.0.0.1:12123".parse()?)?)?;
        let actor_a = system1.spawn_anonymous(Props::new(|| Ok(EmptyTestActor)))?;
        let actor_a = system2.provider().resolve_actor_ref_of_path(actor_a.path());
        let _: MessageToAns = Patterns::ask(&actor_a, MessageToAsk, Duration::from_secs(3)).await?;
        let start = SystemTime::now();
        let range = 0..10000;
        for _ in range {
            let _: MessageToAns =
                Patterns::ask(&actor_a, MessageToAsk, Duration::from_secs(3)).await?;
        }
        let end = SystemTime::now();
        let cost = end.duration_since(start)?;
        info!("cost {:?}", cost);
        let qps = 10000.0 / cost.as_millis() as f64;
        info!("{} per/millis", qps);
        Ok(())
    }
}

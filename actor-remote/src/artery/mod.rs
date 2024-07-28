use std::net::SocketAddr;
use std::sync::Arc;

use ahash::{HashMap, HashMapExt};
use anyhow::anyhow;
use async_trait::async_trait;
use futures::StreamExt;
use quick_cache::unsync::Cache;
use tokio::io::AsyncRead;
use tokio::net::TcpListener;
use tokio_util::codec::FramedRead;
use tracing::{debug, info, warn};

use actor_core::{Actor, DynMessage};
use actor_core::actor::actor_system::ActorSystem;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::coordinated_shutdown::ActorSystemStartFailedReason;
use actor_core::actor_ref::{ActorRef, ActorRefExt};
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::ext::decode_bytes;
use actor_core::message::message_buffer::MessageBufferMap;
use actor_core::message::message_registry::MessageRegistry;
use actor_core::provider::ActorRefProvider;

use crate::artery::codec::PacketCodec;
use crate::artery::connection_status::ConnectionStatus;
use crate::artery::inbound_message::InboundMessage;
use crate::artery::outbound_message::OutboundMessage;
use crate::artery::remote_packet::RemotePacket;
use crate::artery::spawn_inbound::SpawnInbound;
use crate::artery::transport_buffer_envelop::ArteryBufferEnvelope;
use crate::config::advanced::Advanced;
use crate::config::artery::Transport;
use crate::config::message_buffer::MessageBuffer;
use crate::remote_provider::RemoteActorRefProvider;

pub mod connect_quic;
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
pub mod remote_envelope;
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
    provider: Arc<ActorRefProvider>,
    registration: MessageRegistry,
    message_buffer: MessageBufferMap<SocketAddr, ArteryBufferEnvelope>,
}

#[async_trait]
impl Actor for ArteryActor {
    async fn started(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        match self.transport {
            Transport::Tcp => {
                Self::spawn_tcp_listener(context, self.socket_addr)?;
            }
            Transport::TlsTcp => {}
        }
        Ok(())
    }

    async fn on_recv(&mut self, context: &mut ActorContext, message: DynMessage) -> anyhow::Result<()> {
        Self::handle_message(self, context, message).await
    }
}


impl ArteryActor {
    pub(crate) fn new(system: ActorSystem, transport: Transport, socket_addr: SocketAddr, advanced: Advanced) -> anyhow::Result<Self> {
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

    fn resolve_actor_ref(&mut self, path: String) -> ActorRef {
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

    fn buffer_message(
        message_buffer: &mut MessageBufferMap<SocketAddr, ArteryBufferEnvelope>,
        addr: SocketAddr,
        msg: OutboundMessage,
        sender: Option<ActorRef>,
        buffer: MessageBuffer,
    ) {
        let envelope = ArteryBufferEnvelope {
            message: DynMessage::user(msg),
            sender,
        };
        match buffer {
            MessageBuffer::Drop => {
                debug!("no buffer configured, drop buffer message {}", envelope.message.name());
            }
            MessageBuffer::Bound(size) => {
                message_buffer.push(addr, envelope);
                if message_buffer.total_size() > size {
                    for envelope in message_buffer.drop_first_n(&addr, 1) {
                        let msg_name = envelope.message.name();
                        warn!("stash buffer is going to large than {size}, drop the oldest message {msg_name}");
                    }
                }
            }
            MessageBuffer::Unbound => {
                message_buffer.push(addr, envelope);
            }
        }
    }

    fn spawn_tcp_listener(context: &mut ActorContext, addr: SocketAddr) -> anyhow::Result<()> {
        let myself = context.myself().clone();
        let system = context.system().clone();
        context.spawn_fut(format!("tcp_listener_{}", addr), async move {
            info!("start bind tcp addr {}", addr);
            match TcpListener::bind(addr).await {
                Ok(tcp_listener) => {
                    loop {
                        match tcp_listener.accept().await {
                            Ok((stream, peer_addr)) => {
                                let actor = myself.clone();
                                let connection_fut = async move {
                                    ArteryActor::accept_inbound_connection(stream, peer_addr, actor).await;
                                };
                                myself.cast_ns(SpawnInbound { peer_addr, fut: Box::pin(connection_fut) });
                            }
                            Err(error) => {
                                warn!("{} accept connection error {:?}", addr, error);
                            }
                        }
                    }
                }
                Err(error) => {
                    let fut = system.run_coordinated_shutdown(ActorSystemStartFailedReason(anyhow::Error::from(error)));
                    tokio::spawn(fut);
                }
            }
        })?;
        Ok(())
    }


    // fn spawn_quic_listener(
    //     context: &mut ActorContext,
    //     transport: &QuicTransport,
    // ) -> anyhow::Result<()> {
    //     let QuicTransport { addr, config, .. } = transport;
    //     let addr: SocketAddr = (*addr).into();
    //     let myself = context.myself().clone();
    //     let (server_config, _) = config.as_result()?;
    //     let endpoint = Endpoint::server(server_config.clone(), addr)?;
    //     info!("start bind quic addr {}", addr);
    //     context.spawn_fut(format!("quic_listener_{}", addr), async move {
    //         while let Some(conn) = endpoint.accept().await {
    //             match conn.await {
    //                 Ok(connection) => {
    //                     let peer_addr = connection.remote_address();
    //                     match connection.accept_uni().await {
    //                         Ok(stream) => {
    //                             let actor = myself.clone();
    //                             let connection_fut = async move {
    //                                 ArteryActor::accept_inbound_connection(stream, peer_addr, actor).await;
    //                             };
    //                             myself.cast_ns(SpawnInbound { peer_addr, fut: Box::pin(connection_fut) });
    //                         }
    //                         Err(error) => {
    //                             warn!("{} accept connection error {:?}", addr, error);
    //                         }
    //                     }
    //                 }
    //                 Err(error) => {
    //                     warn!("accept connection error {:?}", error);
    //                 }
    //             }
    //         }
    //     })?;
    //     Ok(())
    // }
    //
    // fn configure_client(server_certs: &[&[u8]]) -> anyhow::Result<ClientConfig> {
    //     let mut certs = rustls::RootCertStore::empty();
    //     for cert in server_certs {
    //         certs.add(&rustls::Certificate(cert.to_vec()))?;
    //     }
    //
    //     let client_config = ClientConfig::with_root_certificates(certs);
    //     Ok(client_config)
    // }

    fn is_connecting_or_connected(&self, addr: &SocketAddr) -> bool {
        if let Some(status) = self.connections.get(&addr) {
            match status {
                ConnectionStatus::Connecting(_) |
                ConnectionStatus::Connected(_) => {
                    return true;
                }
                _ => {}
            }
        }
        return false;
    }
}

#[cfg(test)]
mod test {
    use std::net::SocketAddrV4;
    use std::time::{Duration, SystemTime};

    use async_trait::async_trait;
    use bincode::{Decode, Encode};
    use tracing::info;

    use actor_core::{Actor, DynMessage, EmptyTestActor, Message};
    use actor_core::{EmptyCodec, MessageCodec, OrphanCodec};
    use actor_core::actor::actor_system::ActorSystem;
    use actor_core::actor::context::{ActorContext, Context};
    use actor_core::actor::props::Props;
    use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
    use actor_core::actor_ref::ActorRefExt;
    use actor_core::config::actor_setting::ActorSetting;
    use actor_core::pattern::patterns::Patterns;

    use crate::config::message_buffer::MessageBuffer;
    use crate::config::RemoteConfig;
    use crate::config::settings::Settings;
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
            context.spawn_fut("pong", async move {
                sender.cast(Pong, Some(myself));
                tokio::time::sleep(Duration::from_secs(1)).await;
            })?;
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

        async fn on_recv(&mut self, context: &mut ActorContext, message: DynMessage) -> anyhow::Result<()> {
            Self::handle_message(self, context, message).await
        }
    }

    fn build_setting(addr: SocketAddrV4) -> anyhow::Result<ActorSetting> {
        let mut remote_setting = Settings {
            config: RemoteConfig { transport: Transport::tcp(addr, MessageBuffer::default()) },
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
        let actor_a = system_a.spawn(Props::new(|| { Ok(PingPongActor) }), "actor_a")?;
        let system_b = ActorSystem::new("game", build_setting("127.0.0.1:12122".parse()?)?)?;
        let _ = system_b.spawn(Props::new(|| Ok(PingPongActor)), "actor_b")?;
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
            context.sender().unwrap().cast_orphan_ns(MessageToAns {
                content: "hello world".to_string(),
            });
            Ok(())
        }
    }

    #[derive(Encode, Decode, OrphanCodec)]
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
            let _: MessageToAns = Patterns::ask(&actor_a, MessageToAsk, Duration::from_secs(3)).await?;
        }
        let end = SystemTime::now();
        let cost = end.duration_since(start)?;
        info!("cost {:?}", cost);
        let qps = 10000.0 / cost.as_millis() as f64;
        info!("{} per/millis",qps);
        Ok(())
    }
}

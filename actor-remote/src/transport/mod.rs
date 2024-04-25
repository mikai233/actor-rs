use std::net::SocketAddr;
use std::sync::Arc;

use ahash::{HashMap, HashMapExt};
use async_trait::async_trait;
use futures::StreamExt;
use quick_cache::unsync::Cache;
use quinn::{ClientConfig, Endpoint};
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
use actor_core::ext::option_ext::OptionExt;
use actor_core::message::message_buffer::MessageBufferMap;
use actor_core::message::message_registry::MessageRegistry;
use actor_core::provider::{ActorRefProvider, downcast_provider};

use crate::config::buffer::Buffer;
use crate::config::transport::{QuicTransport, Transport};
use crate::remote_provider::RemoteActorRefProvider;
use crate::transport::codec::PacketCodec;
use crate::transport::connection_status::ConnectionStatus;
use crate::transport::inbound_message::InboundMessage;
use crate::transport::outbound_message::OutboundMessage;
use crate::transport::remote_packet::RemotePacket;
use crate::transport::spawn_inbound::SpawnInbound;
use crate::transport::transport_buffer_envelop::TransportBufferEnvelope;

mod codec;
pub(crate) mod remote_envelope;
mod remote_packet;
mod connection_status;
mod connect_tcp;
mod connected;
pub mod disconnect;
pub(crate) mod outbound_message;
mod spawn_inbound;
mod connection;
mod inbound_message;
mod transport_buffer_envelop;
mod disconnected;
mod connect_quic;

pub const ACTOR_REF_CACHE: usize = 10000;

#[derive(Debug)]
pub struct TransportActor {
    transport: Transport,
    connections: HashMap<SocketAddr, ConnectionStatus>,
    actor_ref_cache: Cache<String, ActorRef>,
    provider: Arc<ActorRefProvider>,
    registration: MessageRegistry,
    message_buffer: MessageBufferMap<SocketAddr, TransportBufferEnvelope>,
}

#[async_trait]
impl Actor for TransportActor {
    async fn started(&mut self, context: &mut ActorContext) -> eyre::Result<()> {
        match &self.transport {
            Transport::Tcp(tcp) => {
                Self::spawn_tcp_listener(context, tcp.addr.into());
            }
            Transport::Kcp(_) => {
                unimplemented!("kcp unimplemented");
            }
            Transport::Quic(quic) => {
                Self::spawn_quic_listener(context, quic)?;
            }
        }
        Ok(())
    }
}


impl TransportActor {
    pub(crate) fn new(system: ActorSystem, transport: Transport) -> Self {
        let provider = system.provider_full();
        let remote_provider = downcast_provider::<RemoteActorRefProvider>(&provider);
        let registration = (*remote_provider.registration).clone();
        Self {
            transport,
            connections: HashMap::new(),
            actor_ref_cache: Cache::new(ACTOR_REF_CACHE),
            provider,
            registration,
            message_buffer: Default::default(),
        }
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
        message_buffer: &mut MessageBufferMap<SocketAddr, TransportBufferEnvelope>,
        transport: &Transport,
        addr: SocketAddr,
        msg: OutboundMessage,
        sender: Option<ActorRef>,
    ) {
        let envelope = TransportBufferEnvelope {
            message: DynMessage::user(msg),
            sender,
        };
        match transport.buffer() {
            Buffer::NoBuffer => {
                debug!("no buffer configured, drop buffer message {}", envelope.message.name());
            }
            Buffer::Bound(max_buffer) => {
                message_buffer.push(addr, envelope);
                if message_buffer.total_size() > max_buffer {
                    for envelope in message_buffer.drop_first_n(&addr, 1) {
                        let msg_name = envelope.message.name();
                        warn!("stash buffer is going to large than {max_buffer}, drop the oldest message {msg_name}");
                    }
                }
            }
            Buffer::Unbound => {
                message_buffer.push(addr, envelope);
            }
        }
    }

    fn spawn_tcp_listener(context: &mut ActorContext, addr: SocketAddr) {
        let myself = context.myself().clone();
        let system = context.system().clone();
        context.spawn_fut(async move {
            info!("start bind tcp addr {}", addr);
            match TcpListener::bind(addr).await {
                Ok(tcp_listener) => {
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
                }
                Err(error) => {
                    let fut = system.run_coordinated_shutdown(ActorSystemStartFailedReason(eyre::Error::from(error)));
                    system.handle().spawn(fut);
                }
            }
        });
    }


    fn spawn_quic_listener(
        context: &mut ActorContext,
        transport: &QuicTransport,
    ) -> eyre::Result<()> {
        let QuicTransport { addr, config, .. } = transport;
        let addr: SocketAddr = (*addr).into();
        let myself = context.myself().clone();
        let (server_config, _) = config.as_result()?;
        let endpoint = Endpoint::server(server_config.clone(), addr)?;
        info!("start bind quic addr {}", addr);
        context.spawn_fut(async move {
            while let Some(conn) = endpoint.accept().await {
                match conn.await {
                    Ok(connection) => {
                        let peer_addr = connection.remote_address();
                        match connection.accept_uni().await {
                            Ok(stream) => {
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
                    Err(error) => {
                        warn!("accept connection error {:?}", error);
                    }
                }
            }
        });
        Ok(())
    }

    fn configure_client(server_certs: &[&[u8]]) -> eyre::Result<ClientConfig> {
        let mut certs = rustls::RootCertStore::empty();
        for cert in server_certs {
            certs.add(&rustls::Certificate(cert.to_vec()))?;
        }

        let client_config = ClientConfig::with_root_certificates(certs);
        Ok(client_config)
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
    use actor_core::{EmptyCodec, MessageCodec, OrphanCodec};
    use actor_core::actor::actor_system::ActorSystem;
    use actor_core::actor::context::{ActorContext, Context};
    use actor_core::actor::props::Props;
    use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
    use actor_core::actor_ref::ActorRefExt;
    use actor_core::config::actor_setting::ActorSetting;
    use actor_core::config::ConfigBuilder;
    use actor_core::config::core_config::CoreConfig;
    use actor_core::pattern::patterns::Patterns;

    use crate::config::buffer::Buffer;
    use crate::config::RemoteConfig;
    use crate::config::transport::Transport;
    use crate::remote_provider::RemoteActorRefProvider;
    use crate::remote_setting::RemoteSetting;

    struct PingPongActor;

    #[derive(Encode, Decode, MessageCodec)]
    struct Ping;

    #[async_trait]
    impl Message for Ping {
        type A = PingPongActor;

        async fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut Self::A) -> eyre::Result<()> {
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

        async fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut Self::A) -> eyre::Result<()> {
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

        async fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut Self::A) -> eyre::Result<()> {
            let to = context.system().provider().resolve_actor_ref(&self.to);
            to.cast(Ping, Some(context.myself().clone()));
            Ok(())
        }
    }

    #[async_trait]
    impl Actor for PingPongActor {
        async fn started(&mut self, context: &mut ActorContext) -> eyre::Result<()> {
            info!("{} started", context.myself());
            Ok(())
        }
    }

    fn build_setting(addr: SocketAddrV4) -> eyre::Result<ActorSetting> {
        let mut remote_setting = RemoteSetting {
            config: RemoteConfig { transport: Transport::tcp(addr, Buffer::default()) },
            reg: Default::default(),
        };
        remote_setting.reg.register_user::<Ping>();
        remote_setting.reg.register_user::<Pong>();
        remote_setting.reg.register_user::<MessageToAsk>();
        remote_setting.reg.register_user::<MessageToAns>();
        ActorSetting::new_with_default_config(RemoteActorRefProvider::builder(remote_setting))
    }

    #[tokio::test]
    async fn test() -> eyre::Result<()> {
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

        async fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut Self::A) -> eyre::Result<()> {
            context.sender().unwrap().cast_orphan_ns(MessageToAns {
                content: "hello world".to_string(),
            });
            eyre::Ok(())
        }
    }

    #[derive(Encode, Decode, OrphanCodec)]
    struct MessageToAns {
        content: String,
    }

    #[tokio::test]
    async fn test_remote_ask() -> eyre::Result<()> {
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

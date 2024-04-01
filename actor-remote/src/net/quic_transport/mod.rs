use async_trait::async_trait;
use quinn::{ConnectionError, Endpoint, RecvStream, SendStream, ServerConfig};
use tracing::warn;

use actor_core::Actor;
use actor_core::actor::context::{ActorContext, Context};

use crate::config::transport::QuicTransport;

#[derive(Debug)]
pub struct QuicTransportActor {
    transport: QuicTransport,
}

#[async_trait]
impl Actor for QuicTransportActor {
    async fn started(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        let addr = self.transport.addr;
        let myself = context.myself().clone();
        let (server_config, server_cert) = Self::configure_server()?;
        let endpoint = Endpoint::server(server_config, addr.into())?;
        context.spawn_fut(async move {
            while let Some(conn) = endpoint.accept().await {
                match conn.await {
                    Ok(connection) => {
                        match connection.accept_uni().await {
                            Ok(c) => {}
                            Err(e) => {}
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
}

impl QuicTransportActor {
    fn configure_server() -> anyhow::Result<(ServerConfig, Vec<u8>)> {
        let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
        let cert_der = cert.serialize_der().unwrap();
        let priv_key = cert.serialize_private_key_der();
        let priv_key = rustls::PrivateKey(priv_key);
        let cert_chain = vec![rustls::Certificate(cert_der.clone())];
        let mut server_config = ServerConfig::with_single_cert(cert_chain, priv_key)?;
        Ok((server_config, cert_der))
    }
}
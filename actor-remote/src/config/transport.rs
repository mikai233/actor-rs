use std::net::SocketAddrV4;

use quinn::ServerConfig;
use serde::{Deserialize, Serialize};

use crate::config::buffer::Buffer;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Transport {
    Tcp(TcpTransport),
    Kcp(KcpTransport),
    Quic(QuicTransport),
}

impl Transport {
    pub fn name(&self) -> &'static str {
        match self {
            Transport::Tcp(tcp) => tcp.name(),
            Transport::Kcp(kcp) => kcp.name(),
            Transport::Quic(quic) => quic.name(),
        }
    }

    pub fn addr(&self) -> SocketAddrV4 {
        match self {
            Transport::Tcp(tcp) => tcp.addr,
            Transport::Kcp(_) => {
                unimplemented!("kcp unimplemented");
            }
            Transport::Quic(quic) => quic.addr,
        }
    }

    pub fn buffer(&self) -> Buffer {
        match self {
            Transport::Tcp(tcp) => tcp.buffer,
            Transport::Kcp(_) => {
                unimplemented!("kcp unimplemented");
            }
            Transport::Quic(quic) => quic.buffer,
        }
    }

    pub fn tcp(addr: SocketAddrV4, buffer: Buffer) -> Self {
        Self::Tcp(TcpTransport { addr, buffer })
    }

    pub fn quic(addr: SocketAddrV4, buffer: Buffer, config: (ServerConfig, Vec<u8>)) -> Self {
        Self::Quic(QuicTransport { addr, buffer, config: Some(config) })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcpTransport {
    pub addr: SocketAddrV4,
    pub buffer: Buffer,
}

impl TcpTransport {
    pub fn name(&self) -> &'static str {
        "tcp"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KcpTransport {}

impl KcpTransport {
    pub fn name(&self) -> &'static str {
        "kcp"
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuicTransport {
    pub addr: SocketAddrV4,
    pub buffer: Buffer,
    #[serde(skip)]
    pub config: Option<(ServerConfig, Vec<u8>)>,
}

impl QuicTransport {
    pub fn name(&self) -> &'static str {
        "quic"
    }

    pub fn configure_server() -> anyhow::Result<(ServerConfig, Vec<u8>)> {
        let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
        let cert_der = cert.serialize_der().unwrap();
        let priv_key = cert.serialize_private_key_der();
        let priv_key = rustls::PrivateKey(priv_key);
        let cert_chain = vec![rustls::Certificate(cert_der.clone())];
        let server_config = ServerConfig::with_single_cert(cert_chain, priv_key)?;
        Ok((server_config, cert_der))
    }
}
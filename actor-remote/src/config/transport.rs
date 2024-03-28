use std::net::SocketAddrV4;

use serde::{Deserialize, Serialize};

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

    pub fn tcp(addr: SocketAddrV4, buffer: Option<usize>) -> Self {
        Self::Tcp(TcpTransport { addr, buffer })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcpTransport {
    pub addr: SocketAddrV4,
    pub buffer: Option<usize>,
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
pub struct QuicTransport {}

impl QuicTransport {
    pub fn name(&self) -> &'static str {
        "quic"
    }
}
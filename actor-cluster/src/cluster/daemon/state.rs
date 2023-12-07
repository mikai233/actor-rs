use std::net::SocketAddr;

#[derive(Debug, Copy, Clone)]
pub struct ClusterMember {
    pub addr: SocketAddr,
    pub state: MemberState,
    pub heartbeat: u128,
}

#[derive(Debug, Copy, Clone)]
pub enum MemberState {
    Unknown,
    Connecting,
    Connected,
    Unreachable,
}
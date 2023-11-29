use std::net::SocketAddr;

#[derive(Debug, Copy, Clone)]
pub(crate) struct ClusterMember {
    pub(crate) addr: SocketAddr,
    pub(crate) state: MemberState,
    pub(crate) heartbeat: u128,
}

#[derive(Debug, Copy, Clone)]
pub(crate) enum MemberState {
    Unknown,
    Connecting,
    Connected,
    Unreachable,
}
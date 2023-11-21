use std::collections::HashMap;
use std::net::SocketAddr;

#[derive(Debug)]
pub(crate) struct State {
    pub(crate) members: HashMap<SocketAddr, ClusterMember>,
}

#[derive(Debug, Copy, Clone)]
pub(crate) struct ClusterMember {
    pub(crate) addr: SocketAddr,
    pub(crate) state: MemberState,
    pub(crate) heartbeat: u128,
}

#[derive(Debug, Copy, Clone)]
enum MemberState {
    Unknown,
    Connecting,
    Connected,
    Unreachable,
}
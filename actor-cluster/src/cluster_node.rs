use std::collections::HashSet;
use std::net::SocketAddrV4;

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub(crate) struct ClusterNode {
    pub(crate) nodes: HashSet<SocketAddrV4>,
}
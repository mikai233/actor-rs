use std::sync::RwLock;

use crate::cluster_event::CurrentClusterState;
use crate::cluster_node::ClusterNode;
use crate::member::Member;

#[derive(Debug)]
pub struct ClusterState {
    pub(crate) cluster_node: RwLock<ClusterNode>,
    pub(crate) cluster_state: RwLock<CurrentClusterState>,
    pub(crate) self_member: RwLock<Member>,
}

impl ClusterState {
    pub fn new(state: CurrentClusterState, member: Member) -> Self {
        Self {
            cluster_node: RwLock::new(ClusterNode::default()),
            cluster_state: RwLock::new(state),
            self_member: RwLock::new(member),
        }
    }
}
use std::sync::{Arc, RwLock};

use crate::cluster_event::CurrentClusterState;
use crate::cluster_node::ClusterNode;
use crate::member::Member;

#[derive(Debug, Clone)]
pub struct ClusterState {
    pub cluster_node: Arc<RwLock<ClusterNode>>,
    pub cluster_state: Arc<RwLock<CurrentClusterState>>,
    pub self_member: Arc<RwLock<Member>>,
}

impl ClusterState {
    pub fn new(state: CurrentClusterState, member: Member) -> Self {
        Self {
            cluster_node: Arc::new(RwLock::new(ClusterNode::default())),
            cluster_state: Arc::new(RwLock::new(state)),
            self_member: Arc::new(RwLock::new(member)),
        }
    }
}
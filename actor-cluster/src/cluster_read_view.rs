use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

use crate::cluster_event::{CurrentClusterState, CurrentInternalStats};
use crate::member::Member;
use crate::reachability::Reachability;

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub(crate) struct State {
    pub(crate) cluster_state: CurrentClusterState,
    pub(crate) reachability: Reachability,
    pub(crate) self_member: Member,
    pub(crate) latest_stats: CurrentInternalStats,
}

#[derive(Debug)]
pub(crate) struct ClusterReadView {}
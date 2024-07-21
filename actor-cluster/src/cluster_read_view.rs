use crate::cluster_event::CurrentClusterState;
use crate::member::Member;
use crate::reachability::Reachability;

#[derive(Debug)]
pub(crate) struct State {
    pub(crate) cluster_state: CurrentClusterState,
    pub(crate) reachability: Reachability,
    pub(crate) self_member: Member,
    pub(crate) latest_stats
}

#[derive(Debug)]
pub(crate) struct ClusterReadView {}
use std::time::Duration;

use ahash::{HashMap, HashSet};
use anyhow::{anyhow, ensure};
use bincode::{Decode, Encode};
use config::Config;
use imstr::ImString;
use serde::{Deserialize, Serialize};

use actor_core::actor::address::{ActorPathExtractor, Address};
use actor_core::util::version::Version;

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct ClusterSettings {
    pub failure_detector_implementation_class: ImString,
    pub heartbeat_interval: Duration,
    pub heartbeat_expected_response_after: Duration,
    pub monitored_by_nr_of_members: usize,
    pub seed_nodes: Vec<Address>,
    pub seed_node_timeout: Duration,
    pub retry_unsuccessful_join_after: Duration,
    pub shutdown_after_unsuccessful_join_seed_nodes: Duration,
    pub periodic_tasks_initial_delay: Duration,
    pub gossip_interval: Duration,
    pub gossip_time_to_live: Duration,
    pub leader_actions_interval: Duration,
    pub unreachable_nodes_reaper_interval: Duration,
    pub publish_stats_interval: Duration,
    pub prune_gossip_tombstones_after: Duration,
    pub down_removal_margin: Duration,
    pub downing_provider_class_name: ImString,
    pub quarantine_removed_node_after: Duration,
    pub weakly_up_after: Duration,
    pub allow_weakly_up_members: bool,
    pub self_data_center: ImString,
    pub roles: HashSet<ImString>,
    pub app_version: Version,
    pub min_nr_of_members: usize,
    pub min_nr_of_members_of_role: HashMap<ImString, usize>,
    pub gossip_different_view_probability: f64,
    pub reduce_gossip_different_view_probability: f64,
    pub by_pass_config_compat_check: bool,
    pub config_compat_checkers: HashSet<ImString>,
}

impl ClusterSettings {
    pub fn dc_role_prefix() -> &'static str {
        "dc-"
    }

    pub fn default_data_center() -> &'static str {
        "default"
    }

    pub fn new(config: Config) -> anyhow::Result<Self> {
        let mut cc = config.get_table("akka.cluster")?;
        let mut failure_detector_config = cc
            .remove("failure-detector")
            .ok_or(anyhow!("failure-detector is not found"))?
            .into_table()?;
        let failure_detector_implementation_class = failure_detector_config
            .remove("implementation-class")
            .ok_or(anyhow!("implementation-class is not found"))?
            .into_string()?;
        let heartbeat_interval: Duration = failure_detector_config
            .remove("heartbeat-interval")
            .ok_or(anyhow!("heartbeat-interval is not found"))?
            .try_deserialize()?;
        ensure!(
            heartbeat_interval > Duration::ZERO,
            "heartbeat-interval must be greater than 0"
        );
        let heartbeat_expected_response_after: Duration = failure_detector_config
            .remove("heartbeat-expected-response-after")
            .ok_or(anyhow!("heartbeat-expected-response-after is not found"))?
            .try_deserialize()?;
        ensure!(
            heartbeat_expected_response_after > Duration::ZERO,
            "heartbeat-expected-response-after must be greater than 0"
        );
        let monitored_by_nr_of_members: usize = failure_detector_config
            .remove("monitored-by-nr-of-members")
            .ok_or(anyhow!("monitored-by-nr-of-members is not found"))?
            .try_deserialize()?;
        ensure!(
            monitored_by_nr_of_members > 0,
            "monitored-by-nr-of-members must be greater than 0"
        );
        let mut seed_nodes: Vec<Address> = vec![];
        for ele in cc
            .remove("seed-nodes")
            .ok_or(anyhow!("seed-nodes is not found"))?
            .into_array()?
        {
            let url = ele.into_string()?;
            let (address, _) = ActorPathExtractor::extract(&url)?;
            seed_nodes.push(address);
        }
        let seed_node_timeout: Duration = cc
            .remove("seed-node-timeout")
            .ok_or(anyhow!("seed-node-timeout is not found"))?
            .try_deserialize()?;
        let retry_unsuccessful_join_after = cc
            .remove("retry-unsuccessful-join-after")
            .ok_or(anyhow!("retry-unsuccessful-join-after is not found"))?
            .into_string()?;
        unimplemented!()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct CrossDcFailureDetectorSettings {
    pub implementation_class: String,
    pub heartbeat_interval: Duration,
    pub heartbeat_expected_response_after: Duration,
}

impl CrossDcFailureDetectorSettings {
    pub fn new(config: Config) -> anyhow::Result<Self> {
        let cc = config.get_table("akka.cluster.cross-dc-failure-detector")?;
        unimplemented!("CrossDcFailureDetectorSettings::new")
    }
}

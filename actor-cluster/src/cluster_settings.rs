use std::time::Duration;

use actor_core::util::de_config::de_maybe_off_config;
use ahash::{HashMap, HashMapExt, HashSet};
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
    pub retry_unsuccessful_join_after: Option<Duration>,
    pub shutdown_after_unsuccessful_join_seed_nodes: Option<Duration>,
    pub periodic_tasks_initial_delay: Duration,
    pub gossip_interval: Duration,
    pub gossip_time_to_live: Duration,
    pub leader_actions_interval: Duration,
    pub unreachable_nodes_reaper_interval: Duration,
    pub publish_stats_interval: Option<Duration>,
    pub prune_gossip_tombstones_after: Option<Duration>,
    pub down_removal_margin: Option<Duration>,
    pub downing_provider_class_name: Option<ImString>,
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

    pub fn new(config: &Config) -> anyhow::Result<Self> {
        let mut cc = config.get_table("akka.cluster")?;
        let mut failure_detector_config = cc
            .remove("failure-detector")
            .ok_or(anyhow!("failure-detector is not found"))?
            .into_table()?;
        let failure_detector_implementation_class: ImString = failure_detector_config
            .remove("implementation-class")
            .ok_or(anyhow!("implementation-class is not found"))?
            .into_string()?
            .into();
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
            .ok_or(anyhow!("retry-unsuccessful-join-after is not found"))?;
        let retry_unsuccessful_join_after: Option<Duration> =
            de_maybe_off_config(retry_unsuccessful_join_after)?;
        if let Some(retry_unsuccessful_join_after) = retry_unsuccessful_join_after {
            ensure!(
                retry_unsuccessful_join_after > Duration::ZERO,
                "retry-unsuccessful-join-after must be greater than 0"
            );
        }
        let shutdown_after_unsuccessful_join_seed_nodes = cc
            .remove("shutdown-after-unsuccessful-join-seed-nodes")
            .ok_or(anyhow!(
                "shutdown-after-unsuccessful-join-seed-nodes is not found"
            ))?;
        let shutdown_after_unsuccessful_join_seed_nodes: Option<Duration> =
            de_maybe_off_config(shutdown_after_unsuccessful_join_seed_nodes)?;
        if let Some(shutdown_after_unsuccessful_join_seed_nodes) =
            shutdown_after_unsuccessful_join_seed_nodes
        {
            ensure!(
                shutdown_after_unsuccessful_join_seed_nodes > Duration::ZERO,
                "shutdown-after-unsuccessful-join-seed-nodes must be greater than 0"
            );
        }
        let periodic_tasks_initial_delay: Duration = cc
            .remove("periodic-tasks-initial-delay")
            .ok_or(anyhow!("periodic-tasks-initial-delay is not found"))?
            .try_deserialize()?;
        let gossip_interval: Duration = cc
            .remove("gossip-interval")
            .ok_or(anyhow!("gossip-interval is not found"))?
            .try_deserialize()?;
        let gossip_time_to_live: Duration = cc
            .remove("gossip-time-to-live")
            .ok_or(anyhow!("gossip-time-to-live is not found"))?
            .try_deserialize()?;
        ensure!(
            gossip_time_to_live > Duration::ZERO,
            "gossip-time-to-live must be greater than 0"
        );
        let leader_actions_interval: Duration = cc
            .remove("leader-actions-interval")
            .ok_or(anyhow!("leader-actions-interval is not found"))?
            .try_deserialize()?;
        let unreachable_nodes_reaper_interval: Duration = cc
            .remove("unreachable-nodes-reaper-interval")
            .ok_or(anyhow!("unreachable-nodes-reaper-interval is not found"))?
            .try_deserialize()?;
        let publish_stats_interval = cc
            .remove("publish-stats-interval")
            .ok_or(anyhow!("publish-stats-interval is not found"))?;
        let publish_stats_interval: Option<Duration> = de_maybe_off_config(publish_stats_interval)?;
        if let Some(publish_stats_interval) = publish_stats_interval {
            ensure!(
                publish_stats_interval > Duration::ZERO,
                "publish-stats-interval must be greater than 0"
            );
        }
        let prune_gossip_tombstones_after = cc
            .remove("prune-gossip-tombstones-after")
            .ok_or(anyhow!("prune-gossip-tombstones-after is not found"))?;
        let prune_gossip_tombstones_after: Option<Duration> =
            de_maybe_off_config(prune_gossip_tombstones_after)?;
        if let Some(prune_gossip_tombstones_after) = prune_gossip_tombstones_after {
            ensure!(
                prune_gossip_tombstones_after > Duration::ZERO,
                "prune-gossip-tombstones-after must be greater than 0"
            );
        }
        let down_removal_margin = cc
            .remove("down-removal-margin")
            .ok_or(anyhow!("down-removal-margin is not found"))?;
        let down_removal_margin: Option<Duration> = de_maybe_off_config(down_removal_margin)?;
        if let Some(down_removal_margin) = down_removal_margin {
            ensure!(
                down_removal_margin > Duration::ZERO,
                "down-removal-margin must be greater than 0"
            );
        }
        let mut downing_provider_class_name = cc
            .remove("downing-provider-class-name")
            .ok_or(anyhow!("downing-provider-class-name is not found"))?
            .into_string()?;
        if downing_provider_class_name.is_empty() {
            
        }
        let quarantine_removed_node_after: Duration = cc
            .remove("quarantine-removed-node-after")
            .ok_or(anyhow!("quarantine-removed-node-after is not found"))?
            .try_deserialize()?;
        ensure!(
            quarantine_removed_node_after > Duration::ZERO,
            "quarantine-removed-node-after must be greater than 0"
        );
        let weakly_up_after = cc
            .remove("weakly-up-after")
            .ok_or(anyhow!("weakly-up-after is not found"))?;
        let weakly_up_after: Option<Duration> = de_maybe_off_config(weakly_up_after)?;
        if let Some(weakly_up_after) = weakly_up_after {
            ensure!(
                weakly_up_after > Duration::ZERO,
                "weakly-up-after must be greater than 0"
            );
        }
        let allow_weakly_up_members = weakly_up_after.is_some();
        let self_data_center = cc
            .remove("multi-data-center.self-data-center")
            .ok_or(anyhow!("self-data-center is not found"))?
            .into_string()?;
        let mut roles: HashSet<String> = cc
            .remove("roles")
            .ok_or(anyhow!("roles is not found"))?
            .try_deserialize()?;
        ensure!(
            roles
                .iter()
                .all(|role| !role.starts_with(Self::dc_role_prefix())),
            "Roles mut not start with '{}' as that is reserved for the cluster self-data-center setting",
            Self::dc_role_prefix()
        );
        roles.insert(format!("{}{}", Self::dc_role_prefix(), self_data_center));
        let roles: HashSet<ImString> = roles
            .into_iter()
            .map(|role| ImString::from_std_string(role))
            .collect();
        let app_version = cc
            .remove("app-version")
            .ok_or(anyhow!("app-version is not found"))?
            .into_string()?;
        let app_version = Version::new(app_version)?;
        let min_nr_of_members: usize = cc
            .remove("min-nr-of-members")
            .ok_or(anyhow!("min-nr-of-members is not found"))?
            .try_deserialize()?;
        ensure!(
            min_nr_of_members > 0,
            "min-nr-of-members must be greater than 0"
        );
        let mut min_nr_of_members_of_role: HashMap<String, usize> = HashMap::new();
        let roles = cc
            .remove("role")
            .ok_or(anyhow!("role is not found"))?
            .into_table()?;
        for (role, value) in roles {
            let size: usize = value
                .into_table()?
                .remove("min-nr-of-members-of-role")
                .ok_or(anyhow!("min-nr-of-members-of-role is not found"))?
                .try_deserialize()?;
            min_nr_of_members_of_role.insert(role, size);
        }
        let gossip_different_view_probability: f64 = cc
            .remove("gossip-different-view-probability")
            .ok_or(anyhow!("gossip-different-view-probability is not found"))?
            .try_deserialize()?;
        let reduce_gossip_different_view_probability: f64 = cc
            .remove("reduce-gossip-different-view-probability")
            .ok_or(anyhow!(
                "reduce-gossip-different-view-probability is not found"
            ))?
            .try_deserialize()?;
        let mut configuration_compatibility_check = cc
            .remove("configuration-compatibility-check")
            .ok_or(anyhow!("configuration-compatibility-check is not found"))?
            .into_table()?;
        let by_pass_config_compat_check = configuration_compatibility_check
            .remove("enforce-on-join")
            .ok_or(anyhow!(
                "configuration-compatibility-check.enforce-on-join is not found"
            ))?
            .into_bool()?;
        let checkers = configuration_compatibility_check
            .remove("checkers")
            .ok_or(anyhow!(
                "configuration-compatibility-check.checkers is not found"
            ))?
            .into_table()?;
        let checkers = checkers
            .into_values()
            .map(|v| v.into_string())
            .collect::<Result<HashSet<String>, _>>()?;
        let settings = Self {
            failure_detector_implementation_class,
            heartbeat_interval,
            heartbeat_expected_response_after,
            monitored_by_nr_of_members,
            seed_nodes,
            seed_node_timeout,
            retry_unsuccessful_join_after,
            shutdown_after_unsuccessful_join_seed_nodes,
            periodic_tasks_initial_delay,
            gossip_interval,
            gossip_time_to_live,
            leader_actions_interval,
            unreachable_nodes_reaper_interval,
            publish_stats_interval,
            prune_gossip_tombstones_after,
            down_removal_margin,
            downing_provider_class_name: todo!(),
            quarantine_removed_node_after,
            weakly_up_after: todo!(),
            allow_weakly_up_members,
            self_data_center: todo!(),
            roles: todo!(),
            app_version,
            min_nr_of_members,
            min_nr_of_members_of_role: todo!(),
            gossip_different_view_probability,
            reduce_gossip_different_view_probability,
            by_pass_config_compat_check,
            config_compat_checkers: todo!(),
        };
        Ok(settings)
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

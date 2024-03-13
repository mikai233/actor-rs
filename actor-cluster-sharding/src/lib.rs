pub(crate) const CLUSTER_SHARDING_CONFIG_NAME: &'static str = "cluster-sharding.toml";
pub(crate) const CLUSTER_SHARDING_CONFIG: &'static str = include_str!("../cluster-sharding.toml");

pub mod shard_region;
pub mod cluster_sharding;
mod cluster_sharding_guardian;
mod config;
pub mod shard_coordinator;
mod cluster_sharding_settings;
mod message_extractor;
pub(crate) mod shard;
mod entity_passivation_strategy;
mod handoff_stopper;
pub mod shard_allocation_strategy;

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}

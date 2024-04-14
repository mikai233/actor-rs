use config::{File, FileFormat, Source};
use config::builder::DefaultState;
use serde::{Deserialize, Serialize};

use actor_core::AsAny;
use actor_core::config::{Config, ConfigBuilder};

use crate::CLUSTER_TOOLS_CONFIG;
use crate::config::pub_sub_config::PubSubConfig;
use crate::config::singleton_config::SingletonConfig;
use crate::config::singleton_proxy_config::SingletonProxyConfig;

pub mod singleton_config;
pub mod singleton_proxy_config;
pub mod pub_sub_config;

#[derive(Debug, Clone, Serialize, Deserialize, AsAny)]
pub struct ClusterToolsConfig {
    pub pub_sub: PubSubConfig,
    pub singleton: SingletonConfig,
    pub singleton_proxy: SingletonProxyConfig,
}

impl Config for ClusterToolsConfig {}

#[derive(Debug, Default)]
pub struct ClusterShardingConfigBuilder {
    builder: config::ConfigBuilder<DefaultState>,
}

impl ConfigBuilder for ClusterShardingConfigBuilder {
    type C = ClusterToolsConfig;

    fn add_source<T>(self, source: T) -> eyre::Result<Self> where T: Source + Send + Sync + 'static {
        Ok(Self { builder: self.builder.add_source(source) })
    }

    fn build(self) -> eyre::Result<Self::C> {
        let builder = self.builder.add_source(File::from_str(CLUSTER_TOOLS_CONFIG, FileFormat::Toml));
        let cluster_tools_config = builder.build()?.try_deserialize::<Self::C>()?;
        Ok(cluster_tools_config)
    }
}
use ahash::HashSet;
use config::{File, FileFormat, Source};
use config::builder::DefaultState;
use serde::{Deserialize, Serialize};

use actor_core::AsAny;
use actor_core::config::{Config, ConfigBuilder};
use actor_remote::config::RemoteConfig;

use crate::CLUSTER_CONFIG;

#[derive(Debug, Clone, Serialize, Deserialize, AsAny)]
pub struct ClusterConfig {
    pub remote: RemoteConfig,
    #[serde(default)]
    pub roles: HashSet<String>,
}

impl Config for ClusterConfig {}

impl ClusterConfig {
    pub fn builder() -> ClusterConfigBuilder {
        ClusterConfigBuilder::default()
    }
}

#[derive(Debug, Default)]
pub struct ClusterConfigBuilder {
    builder: config::ConfigBuilder<DefaultState>,
}

impl ConfigBuilder for ClusterConfigBuilder {
    type C = ClusterConfig;

    fn add_source<T>(self, source: T) -> eyre::Result<Self> where T: Source + Send + Sync + 'static {
        Ok(Self { builder: self.builder.add_source(source) })
    }

    fn build(self) -> eyre::Result<Self::C> {
        let builder = self.builder.add_source(File::from_str(CLUSTER_CONFIG, FileFormat::Toml));
        let cluster_config = builder.build()?.try_deserialize::<Self::C>()?;
        Ok(cluster_config)
    }
}


#[cfg(test)]
mod tests {
    use actor_remote::config::buffer::Buffer;
    use actor_remote::config::transport::{TcpTransport, Transport};

    use crate::config::{ClusterConfig, RemoteConfig};

    #[test]
    fn test() {
        let r = RemoteConfig { transport: Transport::Tcp(TcpTransport { addr: "127.0.0.1:8989".parse().unwrap(), buffer: Buffer::default() }) };
        let c = ClusterConfig {
            remote: r,
            roles: Default::default(),
        };
        let r = toml::to_string(&c).unwrap();
        println!("{}", r);
    }
}
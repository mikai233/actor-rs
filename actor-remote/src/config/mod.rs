use config::{File, FileFormat, Source};
use config::builder::DefaultState;
use serde::{Deserialize, Serialize};

use actor_core::AsAny;
use actor_core::config::{Config, ConfigBuilder};

use crate::config::transport::Transport;
use crate::REMOTE_CONFIG;

pub mod transport;
pub mod buffer;

#[derive(Debug, Clone, Serialize, Deserialize, AsAny)]
pub struct RemoteConfig {
    pub transport: Transport,
}

impl Config for RemoteConfig {}

impl RemoteConfig {
    pub fn builder() -> RemoteConfigBuilder {
        RemoteConfigBuilder::default()
    }
}

#[derive(Debug, Default)]
pub struct RemoteConfigBuilder {
    builder: config::ConfigBuilder<DefaultState>,
}

impl ConfigBuilder for RemoteConfigBuilder {
    type C = RemoteConfig;

    fn add_source<T>(self, source: T) -> eyre::Result<Self> where T: Source + Send + Sync + 'static {
        Ok(Self { builder: self.builder.add_source(source) })
    }

    fn build(self) -> eyre::Result<Self::C> {
        let builder = self.builder.add_source(File::from_str(REMOTE_CONFIG, FileFormat::Toml));
        let remote_config = builder.build()?.try_deserialize::<Self::C>()?;
        Ok(remote_config)
    }
}

#[cfg(test)]
mod tests {
    use crate::config::buffer::Buffer;
    use crate::config::RemoteConfig;
    use crate::config::transport::{TcpTransport, Transport};

    #[test]
    fn test() {
        let r = RemoteConfig { transport: Transport::Tcp(TcpTransport { addr: "127.0.0.1:8989".parse().unwrap(), buffer: Buffer::default() }) };
        let r = toml::to_string(&r).unwrap();
        println!("{}", r);
    }
}
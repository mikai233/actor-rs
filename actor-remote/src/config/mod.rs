use serde::{Deserialize, Serialize};
use actor_core::config::Config;


use crate::config::transport::Transport;

pub mod transport;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteConfig {
    pub transport: Transport,
}

impl Config for RemoteConfig {
    fn merge(&self, other: Self) -> Self {
        let RemoteConfig { transport } = other;
        Self {
            transport,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::config::RemoteConfig;
    use crate::config::transport::{TcpTransport, Transport};

    #[test]
    fn test() {
        let r = RemoteConfig { transport: Transport::Tcp(TcpTransport { addr: "127.0.0.1:8989".parse().unwrap(), buffer: None }) };
        let r = toml::to_string(&r).unwrap();
        println!("{}", r);
    }
}
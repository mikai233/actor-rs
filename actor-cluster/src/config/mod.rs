use std::collections::HashSet;
use serde::{Deserialize, Serialize};
use actor_core::config::Config;
use actor_derive::AsAny;
use actor_remote::config::RemoteConfig;

#[derive(Debug, Clone, Serialize, Deserialize, AsAny)]
pub struct ClusterConfig {
    pub remote: RemoteConfig,
    #[serde(default)]
    pub roles: HashSet<String>,
}

impl Config for ClusterConfig {
    fn with_fallback(&self, other: Self) -> Self {
        let ClusterConfig { mut roles, .. } = other;
        roles.extend(self.roles.clone());
        Self {
            remote: self.remote.clone(),
            roles,
        }
    }
}

#[cfg(test)]
mod tests {
    use actor_remote::config::transport::{TcpTransport, Transport};
    use crate::config::{ClusterConfig, RemoteConfig};

    #[test]
    fn test() {
        let r = RemoteConfig { transport: Transport::Tcp(TcpTransport { addr: "127.0.0.1:8989".parse().unwrap(), buffer: None }) };
        let c = ClusterConfig {
            remote: r,
            roles: Default::default(),
        };
        let r = toml::to_string(&c).unwrap();
        println!("{}", r);
    }
}
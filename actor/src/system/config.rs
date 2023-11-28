use std::net::{Ipv4Addr, SocketAddrV4};

use etcd_client::ConnectOptions;

use crate::CodecMessage;
use crate::message::MessageRegistration;

#[derive(Debug, Clone)]
pub struct Config {
    pub name: String,
    pub addr: SocketAddrV4,
    pub registration: MessageRegistration,
    pub etcd_config: EtcdConfig,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            name: "mikai233".to_string(),
            addr: SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 12121),
            registration: MessageRegistration::new(),
            etcd_config: Default::default(),
        }
    }
}

impl Config {
    fn register<M>(&mut self) where M: CodecMessage {
        self.registration.register::<M>()
    }
}

#[derive(Debug, Clone)]
pub struct EtcdConfig {
    pub endpoints: Vec<String>,
    pub connect_options: Option<ConnectOptions>,
}

impl Default for EtcdConfig {
    fn default() -> Self {
        EtcdConfig {
            endpoints: vec!["localhost:2379".to_string()],
            connect_options: None,
        }
    }
}
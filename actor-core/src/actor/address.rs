use std::fmt::{Display, Formatter};
use std::net::SocketAddrV4;

use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(
    Debug,
    Clone,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    Encode,
    Decode,
    Serialize,
    Deserialize
)]
pub struct Address {
    pub protocol: String,
    pub system: String,
    pub addr: Option<SocketAddrV4>,
}

impl Address {
    pub fn new(protocol: impl Into<String>, system: impl Into<String>, addr: Option<SocketAddrV4>) -> Self {
        let protocol = protocol.into();
        let system = system.into();
        Self {
            protocol,
            system,
            addr,
        }
    }
}

impl Display for Address {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match &self.addr {
            None => {
                write!(f, "{}://{}", self.protocol, self.system)?;
            }
            Some(addr) => {
                write!(f, "{}://{}@{}", self.protocol, self.system, addr)?;
            }
        }
        Ok(())
    }
}
use std::fmt::{Display, Formatter};
use std::net::SocketAddrV4;
use std::str::FromStr;

use bincode::{Decode, Encode};
use imstr::ImString;
use serde::{Deserialize, Serialize};

#[derive(
    Debug,
    Copy,
    Clone,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    Encode,
    Decode,
    Serialize,
    Deserialize,
)]
#[repr(u8)]
pub enum Protocol {
    Akka,
}

impl Display for Protocol {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Protocol::Akka => {
                write!(f, "akka")
            }
        }
    }
}

impl FromStr for Protocol {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "akka" => Ok(Protocol::Akka),
            _ => Err(anyhow::anyhow!("Unknown protocol: {}", s)),
        }
    }
}

#[derive(
    Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode, Serialize, Deserialize,
)]
pub struct Address {
    pub protocol: Protocol,
    pub system: ImString,
    pub addr: Option<SocketAddrV4>,
}

impl Address {
    pub fn new(
        protocol: Protocol,
        system: impl Into<ImString>,
        addr: Option<SocketAddrV4>,
    ) -> Self {
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

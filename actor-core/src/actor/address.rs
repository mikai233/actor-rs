use std::fmt::{Display, Formatter};
use std::net::SocketAddrV4;

use bincode::{Decode, Encode};

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode, )]
pub struct Address {
    pub protocol: String,
    pub system: String,
    pub addr: Option<SocketAddrV4>,
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
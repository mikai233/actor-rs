use std::fmt::{Display, Formatter};
use std::net::SocketAddrV4;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Address {
    pub(crate) protocol: String,
    pub(crate) system: String,
    pub(crate) addr: SocketAddrV4,
}

impl Display for Address {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}://{}@{}", self.protocol, self.system, self.addr)?;
        Ok(())
    }
}
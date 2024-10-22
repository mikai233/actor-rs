use std::fmt::{Display, Formatter};
use std::net::SocketAddrV4;

use serde::{Deserialize, Serialize};

use actor_core::actor::address::Address;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct UniqueAddress {
    pub address: Address,
    pub uid: i64,
}

impl UniqueAddress {
    pub fn socket_addr(&self) -> Option<&SocketAddrV4> {
        self.address.addr.as_ref()
    }

    pub fn system_name(&self) -> &str {
        &self.address.system
    }

    pub fn socket_addr_with_uid(&self) -> Option<String> {
        self.socket_addr()
            .map(|addr| format!("{}/{}", addr, self.uid))
    }
}

impl Display for UniqueAddress {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "UniqueAddress({},{})", self.address, self.uid)
    }
}

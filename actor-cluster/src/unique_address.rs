use std::fmt::{Display, Formatter};

use bincode::{Decode, Encode};

use actor_core::actor::address::Address;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub struct UniqueAddress {
    pub address: Address,
    pub uid: i64,
}

impl Display for UniqueAddress {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}[{}]", self.address, self.uid)
    }
}
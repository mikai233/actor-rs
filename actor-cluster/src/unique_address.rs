use bincode::{Decode, Encode};
use actor_core::actor::address::Address;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub struct UniqueAddress {
    pub address: Address,
    pub uid: i64,
}
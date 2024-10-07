use std::collections::VecDeque;
use std::fmt::{Display, Formatter};
use std::sync::atomic::AtomicU64;
use std::sync::Arc;

use crate::actor::address::Address;
use crate::actor_path::{ActorPath, TActorPath};

#[derive(Debug, Clone, derive_more::Deref)]
pub struct RootActorPath(pub(crate) Arc<RootActorPathInner>);

impl Display for RootActorPath {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}{}", self.address, self.name)
    }
}

#[derive(Debug)]
pub struct RootActorPathInner {
    address: Address,
    name: String,
    cached_hash: AtomicU64,
}

impl TActorPath for RootActorPath {
    fn myself(&self) -> ActorPath {
        self.clone().into()
    }

    fn address(&self) -> &Address {
        &self.address
    }

    fn name(&self) -> &String {
        &self.name
    }

    fn parent(&self) -> ActorPath {
        self.clone().into()
    }

    fn elements(&self) -> VecDeque<Arc<String>> {
        let mut v = VecDeque::with_capacity(1);
        v.push_back("".to_string().into());
        v
    }

    fn root(&self) -> &RootActorPath {
        &self
    }

    fn uid(&self) -> i32 {
        ActorPath::undefined_uid()
    }

    fn with_uid(&self, uid: i32) -> ActorPath {
        assert_eq!(
            uid,
            ActorPath::undefined_uid(),
            "RootActorPath must have undefinedUid {} != {}",
            uid,
            ActorPath::undefined_uid()
        );
        ActorPath::RootActorPath(self.clone()).into()
    }

    fn to_string_with_address(&self, address: &Address) -> String {
        match &self.address.addr {
            None => {
                format!("{}{}", address, self.name)
            }
            Some(_) => {
                format!("{}{}", self.address, self.name)
            }
        }
    }

    fn to_serialization_format(&self) -> String {
        self.to_string()
    }
}

impl RootActorPath {
    pub fn new(address: Address, name: impl Into<String>) -> Self {
        let name = name.into();
        assert!(name.len() == 1 || name.rfind('/').unwrap_or_default() == 0,
                "/ may only exist at the beginning of the root actors name, it is a path separator and is not legal in ActorPath names: {}", name);
        assert!(
            name.find('#').is_none(),
            "# is a fragment separator and is not legal in ActorPath names: {}",
            name
        );
        let inner = RootActorPathInner { address, name, cached_hash: AtomicU64::default() };
        Self(inner.into())
    }

    pub(crate) fn cached_hash(&self) -> &AtomicU64 {
        &self.cached_hash
    }
}
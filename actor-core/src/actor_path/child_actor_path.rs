use std::collections::VecDeque;
use std::fmt::{Display, Formatter};
use std::ops::Deref;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;

use crate::actor::address::Address;
use crate::actor_path::{ActorPath, TActorPath};
use crate::actor_path::root_actor_path::RootActorPath;

#[derive(Debug, Clone)]
pub struct ChildActorPath {
    pub(crate) inner: Arc<Inner>,
}

impl Deref for ChildActorPath {
    type Target = Arc<Inner>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Display for ChildActorPath {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match &self.parent {
            ActorPath::RootActorPath(p) => {
                write!(f, "{}{}", p, self.name)
            }
            ActorPath::ChildActorPath(p) => {
                write!(f, "{}/{}", p, self.name)
            }
        }
    }
}

#[derive(Debug)]
pub struct Inner {
    pub(crate) parent: ActorPath,
    pub(crate) name: Arc<String>,
    pub(crate) uid: i32,
    pub(crate) cached_hash: AtomicU64,
}

impl TActorPath for ChildActorPath {
    fn myself(&self) -> ActorPath {
        self.clone().into()
    }

    fn address(&self) -> &Address {
        self.root().address()
    }

    fn name(&self) -> &String {
        &self.name
    }

    fn parent(&self) -> ActorPath {
        self.parent.clone()
    }

    fn elements(&self) -> VecDeque<Arc<String>> {
        let mut queue = VecDeque::with_capacity(10);
        queue.push_front(self.name.clone());
        let mut parent = &self.parent;
        loop {
            match parent {
                ActorPath::RootActorPath(_) => break queue,
                ActorPath::ChildActorPath(c) => {
                    queue.push_front(c.name.clone());
                    parent = &c.parent;
                }
            }
        }
    }

    fn root(&self) -> &RootActorPath {
        let mut parent = &self.parent;
        loop {
            match parent {
                ActorPath::RootActorPath(r) => break r,
                ActorPath::ChildActorPath(c) => {
                    parent = &c.parent;
                }
            }
        }
    }

    fn uid(&self) -> i32 {
        self.uid
    }

    fn with_uid(&self, uid: i32) -> ActorPath {
        if uid == self.uid {
            self.clone().into()
        } else {
            ChildActorPath::new(self.parent.clone(), self.name.clone(), uid).into()
        }
    }

    fn to_string_with_address(&self, address: &Address) -> String {
        let path = self.elements().iter().map(|e| e.as_str()).collect::<Vec<_>>().join("/");
        format!("{}/{}", address, path)
    }

    fn to_serialization_format(&self) -> String {
        let uid = self.uid();
        if uid == ActorPath::undefined_uid() {
            format!("{}", self)
        } else {
            format!("{}#{}", self, uid)
        }
    }
}

impl ChildActorPath {
    pub fn new(parent: ActorPath, name: impl Into<Arc<String>>, uid: i32) -> Self {
        let name = name.into();
        assert!(
            name.find('/').is_none(),
            "/ is a path separator and is not legal in ActorPath names: {}",
            name
        );
        assert!(
            name.find('#').is_none(),
            "# is a fragment separator and is not legal in ActorPath names: {}",
            name
        );
        Self {
            inner: Arc::new(Inner { parent, name, uid, cached_hash: AtomicU64::default() }),
        }
    }

    pub(crate) fn cached_hash(&self) -> &AtomicU64 {
        &self.cached_hash
    }
}
use std::collections::VecDeque;
use std::fmt::{Display, Formatter};
use std::ops::Deref;
use std::sync::Arc;

use crate::actor::actor_path::{ActorPath, TActorPath};
use crate::actor::actor_path::root_actor_path::RootActorPath;
use crate::actor::address::Address;

#[derive(Debug, Clone)]
pub struct ChildActorPath {
    pub(crate) inner: Arc<ChildInner>,
}

impl Deref for ChildActorPath {
    type Target = Arc<ChildInner>;

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

#[derive(Debug, Clone)]
pub struct ChildInner {
    pub(crate) parent: ActorPath,
    pub(crate) name: Arc<String>,
    pub(crate) uid: i32,
}

impl TActorPath for ChildActorPath {
    fn myself(&self) -> ActorPath {
        self.clone().into()
    }

    fn address(&self) -> Address {
        self.root().address()
    }

    fn name(&self) -> &String {
        &self.name
    }

    fn parent(&self) -> ActorPath {
        self.parent.clone()
    }

    fn elements(&self) -> VecDeque<Arc<String>> {
        fn rec(p: ActorPath, mut acc: VecDeque<Arc<String>>) -> VecDeque<Arc<String>> {
            match &p {
                ActorPath::RootActorPath(_) => acc,
                ActorPath::ChildActorPath(c) => {
                    acc.push_front(c.name.clone());
                    rec(p.parent(), acc)
                }
            }
        }
        rec(self.clone().into(), VecDeque::with_capacity(10))
    }

    fn root(&self) -> RootActorPath {
        fn rec(p: ActorPath) -> RootActorPath {
            match &p {
                ActorPath::RootActorPath(r) => r.clone(),
                ActorPath::ChildActorPath(c) => {
                    let a: ActorPath = c.clone().into();
                    rec(a.parent())
                }
            }
        }
        rec(self.clone().into())
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
    pub(crate) fn new(parent: ActorPath, name: impl Into<Arc<String>>, uid: i32) -> Self {
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
            inner: Arc::new(ChildInner { parent, name, uid }),
        }
    }
}
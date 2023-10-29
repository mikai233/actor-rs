use std::collections::VecDeque;
use std::fmt::{Debug, Display, Formatter};
use std::ops::Deref;
use std::sync::Arc;

use enum_dispatch::enum_dispatch;

use crate::actor_ref::SerializedActorRef;
use crate::address::Address;

#[enum_dispatch(ActorPath)]
pub trait TActorPath {
    fn myself(&self) -> ActorPath;
    fn address(&self) -> Address;
    fn name(&self) -> &String;
    fn parent(&self) -> ActorPath;
    fn child(&self, child: String) -> ActorPath {
        let (child_name, uid) = ActorPath::split_name_and_uid(child);
        ChildActorPath {
            inner: Arc::new(ChildInner {
                parent: self.myself(),
                name: child_name,
                uid,
            }),
        }.into()
    }
    fn descendant<I>(&self, names: I) -> ActorPath where I: IntoIterator<Item=String> {
        let names = names.into_iter();
        let init: ActorPath = self.myself();
        names.fold(init, |path, elem| {
            if elem.is_empty() { path } else { path.child(elem) }
        })
    }
    fn elements(&self) -> Vec<String>;
    fn root(&self) -> RootActorPath;
    fn to_serialization(&self) -> SerializedActorRef;
    fn uid(&self) -> i32;
    fn with_uid(&self, uid: i32) -> ActorPath;
}

#[enum_dispatch]
#[derive(Debug, Clone)]
pub enum ActorPath {
    RootActorPath,
    ChildActorPath,
}

impl ActorPath {
    fn undefined_uid() -> i32 {
        0
    }

    fn split_name_and_uid(name: String) -> (String, i32) {
        match name.find('#') {
            None => {
                (name, ActorPath::undefined_uid())
            }
            Some(index) => {
                let (name, id) = (name[0..index].to_string(), &name[(index + 1)..]);
                let id: i32 = id.parse().expect(&format!("expect i32, got {}", id));
                (name, id)
            }
        }
    }
}

impl Display for ActorPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ActorPath::RootActorPath(r) => {
                write!(f, "{}{}", r.address, r.name)
            }
            ActorPath::ChildActorPath(c) => {
                write!(f, "{}/{}", c.parent, c.name)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RootActorPath {
    inner: Arc<RootInner>,
}

impl Deref for RootActorPath {
    type Target = Arc<RootInner>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RootInner {
    address: Address,
    name: String,
}

impl TActorPath for RootActorPath {
    fn myself(&self) -> ActorPath {
        self.clone().into()
    }

    fn address(&self) -> Address {
        self.address.clone()
    }

    fn name(&self) -> &String {
        &self.name
    }

    fn parent(&self) -> ActorPath {
        self.clone().into()
    }

    fn elements(&self) -> Vec<String> {
        vec!["".to_string()]
    }

    fn root(&self) -> RootActorPath {
        self.clone()
    }

    fn to_serialization(&self) -> SerializedActorRef {
        SerializedActorRef {
            address: self.address(),
            path: vec![self.name.clone()],
            uid: ActorPath::undefined_uid(),
        }
    }

    fn uid(&self) -> i32 {
        ActorPath::undefined_uid()
    }

    fn with_uid(&self, uid: i32) -> ActorPath {
        assert_eq!(uid, ActorPath::undefined_uid(), "RootActorPath must have undefinedUid {} != {}", uid, ActorPath::undefined_uid());
        ActorPath::RootActorPath(self.clone()).into()
    }
}

impl RootActorPath {
    pub(crate) fn new(address: Address, name: String) -> Self {
        assert!(name.len() == 1 || name.rfind('/').unwrap_or_default() == 0,
                "/ may only exist at the beginning of the root actors name, it is a path separator and is not legal in ActorPath names: {}", name);
        assert!(name.find('#').is_none(), "# is a fragment separator and is not legal in ActorPath names: {}", name);
        Self {
            inner: Arc::new(
                RootInner {
                    address,
                    name,
                }
            )
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ChildActorPath {
    inner: Arc<ChildInner>,
}

impl Deref for ChildActorPath {
    type Target = Arc<ChildInner>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ChildInner {
    parent: ActorPath,
    name: String,
    uid: i32,
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

    fn elements(&self) -> Vec<String> {
        fn rec(p: ActorPath, mut acc: VecDeque<String>) -> VecDeque<String> {
            match &p {
                ActorPath::RootActorPath(_) => {
                    acc
                }
                ActorPath::ChildActorPath(c) => {
                    acc.push_front(c.name.clone());
                    rec(p.parent(), acc)
                }
            }
        }
        rec(self.clone().into(), VecDeque::new()).into_iter().collect()
    }

    fn root(&self) -> RootActorPath {
        fn rec(p: ActorPath) -> RootActorPath {
            match &p {
                ActorPath::RootActorPath(r) => { r.clone() }
                ActorPath::ChildActorPath(c) => {
                    let a: ActorPath = c.clone().into();
                    rec(a.parent())
                }
            }
        }
        rec(self.clone().into())
    }

    fn to_serialization(&self) -> SerializedActorRef {
        let mut path = vec![self.root().name.clone()];
        path.extend(self.elements());
        SerializedActorRef {
            address: self.address(),
            path,
            uid: self.uid,
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
}

impl ChildActorPath {
    pub(crate) fn new(parent: ActorPath, name: String, uid: i32) -> Self {
        assert!(name.find('/').is_none(), "/ is a path separator and is not legal in ActorPath names: {}", name);
        assert!(name.find('#').is_none(), "# is a fragment separator and is not legal in ActorPath names: {}", name);
        Self {
            inner: Arc::new(
                ChildInner {
                    parent,
                    name,
                    uid,
                }
            )
        }
    }
}

#[cfg(test)]
mod actor_path_test {
    use crate::actor_path::{ActorPath, ChildActorPath, RootActorPath, TActorPath};
    use crate::address::Address;

    fn get_address() -> Address {
        Address {
            protocol: "akka".to_string(),
            system: "game_server".to_string(),
            host: Some("127.0.0.1".to_string()),
            port: Some(12121),
        }
    }

    #[test]
    #[should_panic]
    fn test_root_actor_panic() {
        let addr = get_address();
        RootActorPath::new(addr, "/a/b".to_string());
    }

    #[test]
    #[should_panic]
    fn test_root_actor_panic2() {
        let addr = get_address();
        RootActorPath::new(addr, "#a".to_string());
    }

    #[test]
    fn test_root_actor() {
        let addr = get_address();
        RootActorPath::new(addr.clone(), "/a".to_string());
        let root = RootActorPath::new(addr, "a".to_string());
        root.with_uid(ActorPath::undefined_uid());
    }

    #[test]
    #[should_panic]
    fn test_child_actor_panic() {
        let addr = get_address();
        let root = RootActorPath::new(addr, "a".to_string()).into();
        ChildActorPath::new(root, "b#".to_string(), 11233);
    }

    #[test]
    #[should_panic]
    fn test_child_actor_panic2() {
        let addr = get_address();
        let root = RootActorPath::new(addr, "a".to_string()).into();
        ChildActorPath::new(root, "b/".to_string(), 11233);
    }

    #[test]
    fn test_actor_path() {
        let addr = get_address();
        let root: ActorPath = RootActorPath::new(addr, "/a".to_string()).into();
        let child = root.child("b121#112223289".to_string());
        assert_eq!(child.to_string(), format!("{}/a/b121", get_address()));
        let c1 = child.child("child1#-12324".to_string());
        assert_eq!(c1.to_string(), format!("{}/a/b121/child1", get_address()));
        let c2 = child.child("child2#3249238".to_string());
        assert_eq!(c2.to_string(), format!("{}/a/b121/child2", get_address()));
    }
}
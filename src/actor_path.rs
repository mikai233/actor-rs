use std::cmp::Ordering;
use std::fmt::{Debug, Display, Formatter};
use std::ops::Deref;
use std::sync::Arc;

use crate::address::Address;

#[derive(Debug, Clone)]
pub enum ActorPath {
    Root(RootActorPath),
    Child(ChildActorPath),
}

impl ActorPath {
    pub(crate) fn address(&self) -> Address {
        match self {
            ActorPath::Root(r) => { r.address() }
            ActorPath::Child(c) => { c.address() }
        }
    }
    pub(crate) fn name(&self) -> &String {
        match self {
            ActorPath::Root(r) => {
                r.name()
            }
            ActorPath::Child(c) => {
                c.name()
            }
        }
    }
    pub(crate) fn parent(&self) -> ActorPath {
        match self {
            ActorPath::Root(r) => {
                r.parent()
            }
            ActorPath::Child(c) => {
                c.parent()
            }
        }
    }
    pub(crate) fn child(&self, child: &String) -> ActorPath {
        let (child_name, uid) = split_name_and_uid(child);
        ActorPath::Child(ChildActorPath::new(self.clone(), child_name, uid))
    }
    pub(crate) fn root(&self) -> RootActorPath {
        match self {
            ActorPath::Root(r) => {
                r.root()
            }
            ActorPath::Child(c) => {
                c.root()
            }
        }
    }
    pub(crate) fn uid(&self) -> i32 {
        match self {
            ActorPath::Root(r) => {
                r.uid()
            }
            ActorPath::Child(c) => {
                c.uid()
            }
        }
    }
    pub(crate) fn with_uid(&self, uid: i32) -> ActorPath {
        match self {
            ActorPath::Root(r) => {
                r.with_uid(uid)
            }
            ActorPath::Child(c) => {
                c.with_uid(uid)
            }
        }
    }
}

impl Display for ActorPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ActorPath::Root(r) => {
                std::fmt::Display::fmt(&r, f)
            }
            ActorPath::Child(c) => {
                std::fmt::Display::fmt(&c, f)
            }
        }
    }
}

impl PartialEq for ActorPath {
    fn eq(&self, other: &Self) -> bool {
        match self {
            ActorPath::Root(r) => {
                match other {
                    ActorPath::Root(or) => {
                        r.eq(or)
                    }
                    ActorPath::Child(_) => {
                        false
                    }
                }
            }
            ActorPath::Child(c) => {
                match other {
                    ActorPath::Root(_) => {
                        false
                    }
                    ActorPath::Child(oc) => {
                        c.eq(oc)
                    }
                }
            }
        }
    }
}

impl Eq for ActorPath {}

impl PartialOrd for ActorPath {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self {
            ActorPath::Root(r) => {
                match other {
                    ActorPath::Root(or) => {
                        r.partial_cmp(or)
                    }
                    ActorPath::Child(_) => {
                        Some(Ordering::Greater)
                    }
                }
            }
            ActorPath::Child(c) => {
                match other {
                    ActorPath::Root(_) => {
                        Some(Ordering::Less)
                    }
                    ActorPath::Child(oc) => {
                        c.partial_cmp(&oc)
                    }
                }
            }
        }
    }
}

impl Ord for ActorPath {
    fn cmp(&self, other: &Self) -> Ordering {
        match self {
            ActorPath::Root(r) => {
                match other {
                    ActorPath::Root(or) => {
                        r.cmp(or)
                    }
                    ActorPath::Child(_) => {
                        Ordering::Greater
                    }
                }
            }
            ActorPath::Child(c) => {
                match other {
                    ActorPath::Root(_) => {
                        Ordering::Less
                    }
                    ActorPath::Child(oc) => {
                        c.cmp(&oc)
                    }
                }
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

    pub(crate) fn address(&self) -> Address {
        self.address.clone()
    }

    pub(crate) fn name(&self) -> &String {
        &self.name
    }

    pub(crate) fn parent(&self) -> ActorPath {
        ActorPath::Root(self.clone())
    }

    pub(crate) fn root(&self) -> RootActorPath {
        self.clone()
    }

    pub(crate) fn uid(&self) -> i32 {
        UNDEFINED_UID
    }

    pub(crate) fn with_uid(&self, uid: i32) -> ActorPath {
        assert_eq!(uid, UNDEFINED_UID, "RootActorPath must have undefinedUid {} != {}", uid, UNDEFINED_UID);
        ActorPath::Root(self.clone())
    }
}

impl Display for RootActorPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.address, self.name)
    }
}

impl PartialEq for RootActorPath {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for RootActorPath {}

impl PartialOrd for RootActorPath {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.to_string().partial_cmp(&other.to_string())
    }
}

impl Ord for RootActorPath {
    fn cmp(&self, other: &Self) -> Ordering {
        self.to_string().cmp(&other.to_string())
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

    pub(crate) fn address(&self) -> Address {
        self.root().address()
    }

    fn name(&self) -> &String {
        &self.name
    }

    fn parent(&self) -> ActorPath {
        self.parent.clone()
    }

    fn root(&self) -> RootActorPath {
        match self.parent() {
            ActorPath::Root(r) => {
                r
            }
            ActorPath::Child(c) => {
                c.parent().root()
            }
        }
    }

    fn uid(&self) -> i32 {
        self.uid
    }

    fn with_uid(&self, uid: i32) -> ActorPath {
        if uid == self.uid {
            ActorPath::Child(self.clone())
        } else {
            ActorPath::Child(ChildActorPath::new(self.parent.clone(), self.name.clone(), uid))
        }
    }
}

impl Display for ChildActorPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.parent, self.name)
    }
}

impl PartialEq for ChildActorPath {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.parent.eq(&other.parent)
    }
}

impl Eq for ChildActorPath {}

impl PartialOrd for ChildActorPath {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self.name.partial_cmp(&other.name) {
            None => {
                None
            }
            Some(o) => {
                if matches!(o,Ordering::Equal) {
                    self.parent.partial_cmp(&other.parent)
                } else {
                    Some(o)
                }
            }
        }
    }
}

impl Ord for ChildActorPath {
    fn cmp(&self, other: &Self) -> Ordering {
        let order = self.name.cmp(&other.name);
        if matches!(order,Ordering::Equal) {
            self.parent.cmp(&other.parent)
        } else {
            order
        }
    }
}

const UNDEFINED_UID: i32 = 0;

fn split_name_and_uid(name: &String) -> (String, i32) {
    match name.find('#') {
        None => {
            (name.clone(), UNDEFINED_UID)
        }
        Some(index) => {
            let (name, id) = (name[0..index].to_string(), &name[(index + 1)..]);
            let id: i32 = id.parse().expect(&format!("expect i32, got {}", id));
            (name, id)
        }
    }
}

#[cfg(test)]
mod actor_path_test {
    use crate::actor_path::{ActorPath, ChildActorPath, RootActorPath, UNDEFINED_UID};
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
        root.with_uid(UNDEFINED_UID);
    }

    #[test]
    #[should_panic]
    fn test_child_actor_panic() {
        let addr = get_address();
        let root = ActorPath::Root(RootActorPath::new(addr, "a".to_string()));
        ChildActorPath::new(root, "b#".to_string(), 11233);
    }

    #[test]
    #[should_panic]
    fn test_child_actor_panic2() {
        let addr = get_address();
        let root = ActorPath::Root(RootActorPath::new(addr, "a".to_string()));
        ChildActorPath::new(root, "b/".to_string(), 11233);
    }

    #[test]
    fn test_actor_path() {
        let addr = get_address();
        let root = ActorPath::Root(RootActorPath::new(addr, "/a".to_string()));
        let child = root.child(&"b121#112223289".to_string());
        assert_eq!(child.to_string(), format!("{}/a/b121", get_address()));
        let c1 = child.child(&"child1#-12324".to_string());
        assert_eq!(c1.to_string(), format!("{}/a/b121/child1", get_address()));
        let c2 = child.child(&"child2#3249238".to_string());
        assert_eq!(c2.to_string(), format!("{}/a/b121/child2", get_address()));
    }

    #[test]
    fn test_actor_path_cmp() {
        let addr = get_address();
        let root1 = ActorPath::Root(RootActorPath::new(addr.clone(), "/a".to_string()));
        let root2 = ActorPath::Root(RootActorPath::new(addr.clone(), "/a".to_string()));
        assert_eq!(root1, root2);
        let root2 = ActorPath::Root(RootActorPath::new(addr.clone(), "/ab".to_string()));
        assert!(root2 > root1);
        assert!(root1 < root2);
        let child1 = root1.child(&"b".to_string());
        let child2 = root2.child(&"b".to_string());
        assert!(child1 < child2);
        let child3 = child1.child(&"c".to_string());
        let child4 = child2.child(&"b".to_string());
        assert!(child3 > child4);
        assert!(root1 > child3);
        assert!(child3 < root1);
    }
}
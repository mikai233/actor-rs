use std::cmp::Ordering;
use std::collections::VecDeque;
use std::fmt::{Display, Formatter};
use std::format;
use std::hash::{Hash, Hasher};
use std::net::SocketAddrV4;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;

use anyhow::{anyhow, Context};
use enum_dispatch::enum_dispatch;
use rand::random;
use url::Url;

use crate::actor::address::{Address, Protocol};
use crate::actor_path::child_actor_path::{ChildActorPath, Inner};
use crate::actor_path::root_actor_path::RootActorPath;

pub mod child_actor_path;
pub mod root_actor_path;

#[enum_dispatch(ActorPath)]
pub trait TActorPath {
    fn myself(&self) -> ActorPath;

    fn address(&self) -> &Address;

    fn name(&self) -> &String;

    fn parent(&self) -> ActorPath;

    fn child(&self, child: &str) -> ActorPath {
        let (child_name, uid) = ActorPath::split_name_and_uid(&child);
        ChildActorPath {
            inner: Arc::new(Inner {
                parent: self.myself(),
                name: child_name.into(),
                uid,
                cached_hash: AtomicU64::default(),
            }),
        }.into()
    }

    fn descendant<'a, I>(&self, names: I) -> ActorPath
        where
            I: IntoIterator<Item=&'a str>,
    {
        let init: ActorPath = self.myself();
        names.into_iter().fold(init, |path, elem| {
            if elem.is_empty() {
                path
            } else {
                path.child(elem)
            }
        })
    }

    fn elements(&self) -> VecDeque<Arc<String>>;

    fn root(&self) -> &RootActorPath;

    fn uid(&self) -> i32;

    fn with_uid(&self, uid: i32) -> ActorPath;

    fn to_string_without_address(&self) -> String {
        self.elements().iter().map(|e| e.as_str()).collect::<Vec<_>>().join("/")
    }

    fn to_string_with_address(&self, address: &Address) -> String;

    fn to_serialization_format(&self) -> String;

    fn to_serialization_format_with_address(&self, address: &Address) -> String {
        let uid = self.uid();
        if uid == ActorPath::undefined_uid() {
            format!("{}", self.to_string_with_address(address))
        } else {
            format!("{}#{}", self.to_string_with_address(address), uid)
        }
    }
}

#[enum_dispatch]
#[derive(Debug, Clone)]
pub enum ActorPath {
    RootActorPath,
    ChildActorPath,
}

impl PartialEq for ActorPath {
    fn eq(&self, other: &Self) -> bool {
        let self_address = self.address();
        let other_address = other.address();
        if self_address != other_address {
            return false;
        }
        let self_elements = self.elements();
        let other_elements = other.elements();
        self_elements == other_elements
    }
}

impl Eq for ActorPath {}

impl PartialOrd for ActorPath {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let self_elements = self.elements();
        let self_address = self.address();
        let other_elements = other.elements();
        let other_address = other.address();
        let order = self_address.partial_cmp(&other_address);
        match &order {
            Some(Ordering::Equal) => {
                self_elements.partial_cmp(&other_elements)
            }
            _ => order
        }
    }
}

impl Ord for ActorPath {
    fn cmp(&self, other: &Self) -> Ordering {
        let self_elements = self.elements();
        let self_address = self.address();
        let other_elements = other.elements();
        let other_address = other.address();
        let order = self_address.cmp(&other_address);
        match &order {
            Ordering::Equal => {
                self_elements.cmp(&other_elements)
            }
            _ => order
        }
    }
}

impl Hash for ActorPath {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let cached_hash = match self {
            ActorPath::RootActorPath(r) => { r.cached_hash() }
            ActorPath::ChildActorPath(c) => { c.cached_hash() }
        };
        let hash = cached_hash.load(std::sync::atomic::Ordering::Relaxed);
        if hash != 0 {
            state.write_u64(hash);
        } else {
            let mut hasher = ahash::AHasher::default();
            self.address().hash(&mut hasher);
            for e in self.elements() {
                e.hash(&mut hasher);
            }
            let hash = hasher.finish();
            cached_hash.store(hash, std::sync::atomic::Ordering::Relaxed);
            state.write_u64(hash);
        }
    }
}

impl ActorPath {
    pub(crate) fn undefined_uid() -> i32 {
        0
    }

    pub(crate) fn new_uid() -> i32 {
        let uid = random::<i32>();
        if uid == ActorPath::undefined_uid() {
            ActorPath::new_uid()
        } else {
            uid
        }
    }

    pub(crate) fn split_name_and_uid(name: &str) -> (String, i32) {
        match name.find('#') {
            None => (name.to_string(), ActorPath::undefined_uid()),
            Some(index) => {
                let (name, id) = (name[0..index].to_string(), &name[(index + 1)..]);
                let id: i32 = id.parse().expect(&format!("expect i32, got {}", id));
                (name, id)
            }
        }
    }
}

impl Display for ActorPath {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            ActorPath::RootActorPath(r) => {
                write!(f, "{}", r)
            }
            ActorPath::ChildActorPath(c) => {
                write!(f, "{}", c)
            }
        }
    }
}

impl FromStr for ActorPath {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let url = Url::parse(s).context(format!("invalid url {}", s))?;
        let scheme = Protocol::from_str(url.scheme())?;
        let username = url.username().into();
        let host = url.domain().ok_or(anyhow!("no domain found in url {}", s))?;
        let port = url.port().ok_or(anyhow!("no port found in url {}", s))?;
        let addr: SocketAddrV4 = format!("{}:{}", host, port).parse()?;
        let mut path_str = url
            .path()
            .split("/")
            .map(|s| s.to_string())
            .collect::<Vec<_>>();
        path_str.remove(0);
        let uid: i32 = url.fragment().unwrap_or("0").parse()?;
        let address = Address {
            protocol: scheme,
            system: username,
            addr: Some(addr),
        };
        let mut path: ActorPath = RootActorPath::new(address, "/").into();
        for p in path_str {
            path = path.child(&p);
        }
        path = path.with_uid(uid);
        Ok(path)
    }
}


#[cfg(test)]
mod test {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    use anyhow::Ok;

    use crate::actor::address::{Address, Protocol};
    use crate::actor_path::{ActorPath, TActorPath};
    use crate::actor_path::child_actor_path::ChildActorPath;
    use crate::actor_path::root_actor_path::RootActorPath;

    fn build_address() -> Address {
        Address::new(Protocol::Akka, "mikai233", Some("127.0.0.1:12121".parse().unwrap()))
    }

    fn build_actor_path() -> ActorPath {
        let addr = build_address();
        let root: ActorPath = RootActorPath::new(addr, "/".to_string()).into();
        let actor_path = root.descendant(vec![
            "user".to_string(),
            "$a".to_string(),
            "$a".to_string(),
            format!("$aa#{}", ActorPath::new_uid()),
        ].iter().map(|e| e.as_str()));
        actor_path
    }

    #[test]
    #[should_panic]
    fn test_root_actor_panic() {
        let addr = build_address();
        RootActorPath::new(addr, "/a/b");
    }

    #[test]
    #[should_panic]
    fn test_root_actor_panic2() {
        let addr = build_address();
        RootActorPath::new(addr, "#a");
    }

    #[test]
    fn test_root_actor() {
        let addr = build_address();
        RootActorPath::new(addr.clone(), "/a");
        let root = RootActorPath::new(addr, "a");
        root.with_uid(ActorPath::undefined_uid());
    }

    #[test]
    #[should_panic]
    fn test_child_actor_panic() {
        let addr = build_address();
        let root = RootActorPath::new(addr, "a").into();
        ChildActorPath::new(root, "b#".to_string(), 11233);
    }

    #[test]
    #[should_panic]
    fn test_child_actor_panic2() {
        let addr = build_address();
        let root = RootActorPath::new(addr, "a").into();
        ChildActorPath::new(root, "b/".to_string(), 11233);
    }

    #[test]
    fn test_actor_path() {
        let addr = build_address();
        let root: ActorPath = RootActorPath::new(addr, "/".to_string()).into();
        let child = root.child(&"b121#112223289");
        assert_eq!(child.to_string(), format!("{}/b121", build_address()));
        let c1 = child.child(&"child1#-12324");
        assert_eq!(c1.to_string(), format!("{}/b121/child1", build_address()));
        let c2 = child.child(&"child2#3249238");
        assert_eq!(c2.to_string(), format!("{}/b121/child2", build_address()));
    }

    #[test]
    fn test_actor_path_serde() -> anyhow::Result<()> {
        let actor_path = build_actor_path();
        let url = actor_path.to_serialization_format();
        let parse_path: ActorPath = url.parse()?;
        assert_eq!(actor_path, parse_path);
        Ok(())
    }

    #[test]
    fn test_to_string_without_address() {
        let addr = build_address();
        let root: ActorPath = RootActorPath::new(addr, "/".to_string()).into();
        assert_eq!(root.to_string_without_address(), "");
        let actor_path = build_actor_path();
        assert_eq!(actor_path.to_string_without_address(), "user/$a/$a/$aa");
    }

    #[test]
    fn test_to_string_with_address() {
        let actor_path = build_actor_path();
        let address = Address::new(Protocol::Akka, "mikai", None);
        assert_eq!(actor_path.to_string_with_address(&address), "tcp://mikai/user/$a/$a/$aa");
        let address = Address::new(Protocol::Akka, "mikai", Some("127.0.0.1:9988".parse().unwrap()));
        assert_eq!(actor_path.to_string_with_address(&address), "tcp://mikai@127.0.0.1:9988/user/$a/$a/$aa");
    }

    #[test]
    fn test_eq() {
        let actor_path = build_actor_path();
        assert_eq!(actor_path, actor_path);
        assert_ne!(actor_path, actor_path.child("u"));
        let addr = Address::new(Protocol::Akka, "mikai233", None);
        let root: ActorPath = RootActorPath::new(addr, "/".to_string()).into();
        let actor_path2 = root.descendant(vec![
            "user".to_string(),
            "$a".to_string(),
            "$a".to_string(),
            format!("$aa#{}", ActorPath::new_uid()),
        ].iter().map(|e| e.as_str()));
        assert_ne!(actor_path, actor_path2);
    }

    #[test]
    fn test_order() {
        let actor_path = build_actor_path();
        assert!(!(actor_path > actor_path));
        assert!(!(actor_path < actor_path));
        let actor_path2 = actor_path.child("u");
        assert!(actor_path < actor_path2);
        let actor_path3 = actor_path.child("a");
        assert!(actor_path3 < actor_path2);
        let addr = Address::new(Protocol::Akka, "mikai234", None);
        let root: ActorPath = RootActorPath::new(addr, "/".to_string()).into();
        let actor_path4 = root.descendant(vec![
            "user".to_string(),
            "$a".to_string(),
            "$a".to_string(),
            format!("$aa#{}", ActorPath::new_uid()),
        ].iter().map(|e| e.as_str()));
        assert!(actor_path < actor_path4);
    }

    #[test]
    fn test_hash() {
        let mut hasher = DefaultHasher::new();
        let actor_path = build_actor_path();
        actor_path.hash(&mut hasher);
        let h1 = hasher.finish();
        let mut hasher = DefaultHasher::new();
        actor_path.hash(&mut hasher);
        let h2 = hasher.finish();
        assert_eq!(h1, h2);
        let actor_path2 = actor_path.child("a");
        let mut hasher = DefaultHasher::new();
        actor_path2.hash(&mut hasher);
        let h3 = hasher.finish();
        assert_ne!(h1, h3);
    }
}
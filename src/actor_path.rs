use std::collections::VecDeque;
use std::fmt::{Debug, Display, Formatter};
use std::net::SocketAddrV4;
use std::ops::Deref;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::{anyhow, Context};
use enum_dispatch::enum_dispatch;
use rand::random;
use url::Url;

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
        }
            .into()
    }
    fn descendant<I>(&self, names: I) -> ActorPath
        where
            I: IntoIterator<Item=String>,
    {
        let names = names.into_iter();
        let init: ActorPath = self.myself();
        names.fold(init, |path, elem| {
            if elem.is_empty() {
                path
            } else {
                path.child(elem)
            }
        })
    }
    fn elements(&self) -> Vec<String>;
    fn root(&self) -> RootActorPath;
    fn uid(&self) -> i32;
    fn with_uid(&self, uid: i32) -> ActorPath;
}

#[enum_dispatch]
#[derive(Debug, Clone)]
pub enum ActorPath {
    RootActorPath,
    ChildActorPath,
}

impl PartialEq for ActorPath {
    fn eq(&self, other: &Self) -> bool {
        self.to_string().eq(&other.to_string())
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

    pub(crate) fn split_name_and_uid(name: String) -> (String, i32) {
        match name.find('#') {
            None => (name, ActorPath::undefined_uid()),
            Some(index) => {
                let (name, id) = (name[0..index].to_string(), &name[(index + 1)..]);
                let id: i32 = id.parse().expect(&format!("expect i32, got {}", id));
                (name, id)
            }
        }
    }

    pub fn to_serialization(&self) -> String {
        let uid = self.uid();
        if uid == ActorPath::undefined_uid() {
            self.to_string()
        } else {
            format!("{}#{}", self, uid)
        }
    }
}

impl Display for ActorPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ActorPath::RootActorPath(r) => {
                write!(f, "{}{}", r.address, r.name)
            }
            ActorPath::ChildActorPath(c) => match &c.parent {
                ActorPath::RootActorPath(_) => {
                    write!(f, "{}{}", c.parent, c.name)
                }
                ActorPath::ChildActorPath(_) => {
                    write!(f, "{}/{}", c.parent, c.name)
                }
            },
        }
    }
}

impl FromStr for ActorPath {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let url = Url::parse(s).context(format!("invalid url {}", s))?;
        let scheme = url.scheme().to_string();
        let username = url.username().to_string();
        let host = url.domain().ok_or(anyhow!("no host found in url {}", s))?;
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
            addr,
        };
        let mut path: ActorPath = RootActorPath::new(address, "/".to_string()).into();
        for p in path_str {
            path = path.child(p);
        }
        path = path.with_uid(uid);
        Ok(path)
    }
}

#[derive(Debug, Clone)]
pub struct RootActorPath {
    pub inner: Arc<RootInner>,
}

impl Deref for RootActorPath {
    type Target = Arc<RootInner>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Debug, Clone)]
pub struct RootInner {
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
}

impl RootActorPath {
    pub(crate) fn new(address: Address, name: String) -> Self {
        assert!(name.len() == 1 || name.rfind('/').unwrap_or_default() == 0,
                "/ may only exist at the beginning of the root actors name, it is a path separator and is not legal in ActorPath names: {}", name);
        assert!(
            name.find('#').is_none(),
            "# is a fragment separator and is not legal in ActorPath names: {}",
            name
        );
        Self {
            inner: Arc::new(RootInner { address, name }),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChildActorPath {
    inner: Arc<ChildInner>,
}

impl Deref for ChildActorPath {
    type Target = Arc<ChildInner>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Debug, Clone)]
pub struct ChildInner {
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
                ActorPath::RootActorPath(_) => acc,
                ActorPath::ChildActorPath(c) => {
                    acc.push_front(c.name.clone());
                    rec(p.parent(), acc)
                }
            }
        }
        rec(self.clone().into(), VecDeque::new())
            .into_iter()
            .collect()
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
}

impl ChildActorPath {
    pub(crate) fn new(parent: ActorPath, name: String, uid: i32) -> Self {
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

#[cfg(test)]
mod actor_path_test {
    use anyhow::Ok;

    use crate::actor_path::{ActorPath, ChildActorPath, RootActorPath, TActorPath};
    use crate::address::Address;

    fn get_address() -> Address {
        Address {
            protocol: "tcp".to_string(),
            system: "game_server".to_string(),
            addr: "127.0.0.1:12121".parse().unwrap(),
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

    #[test]
    fn test_actor_path_serde() -> anyhow::Result<()> {
        let addr = get_address();
        let root: ActorPath = RootActorPath::new(addr, "/".to_string()).into();
        let actor_path = root.descendant(vec![
            "user".to_string(),
            "$a".to_string(),
            "$a".to_string(),
            format!("$aa#{}", ActorPath::new_uid()),
        ]);
        let url = actor_path.to_serialization();
        let parse_path: ActorPath = url.parse()?;
        assert_eq!(actor_path, parse_path);
        Ok(())
    }
}

use std::collections::VecDeque;
use std::fmt::{Debug, Display, Formatter};
use std::net::SocketAddrV4;
use std::str::FromStr;

use anyhow::{anyhow, Context};
use enum_dispatch::enum_dispatch;
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;
use url::Url;

use crate::actor::Message;
use crate::actor_path::ActorPath;
use crate::actor_path::TActorPath;
use crate::actor_ref::dead_letter_ref::DeadLetterActorRef;
use crate::actor_ref::local_ref::LocalActorRef;
use crate::actor_ref::remote_ref::RemoteActorRef;
use crate::address::Address;
use crate::message::{ActorMessage, MessageID};
use crate::system::ActorSystem;

pub mod local_ref;
pub mod remote_ref;
pub mod dead_letter_ref;

#[enum_dispatch]
#[derive(Debug, Clone)]
pub enum ActorRef {
    LocalActorRef,
    RemoteActorRef,
    DeadLetterActorRef,
}

impl ActorRef {
    fn no_sender() -> Option<ActorRef> {
        None
    }
}

#[enum_dispatch(ActorRef)]
pub trait TActorRef: Debug + Send + Sync + 'static {
    fn system(&self) -> ActorSystem;
    fn path(&self) -> &ActorPath;
    fn tell(&self, message: ActorMessage, sender: Option<ActorRef>);
}

impl Display for ActorRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let path = self.path();
        let uid = path.uid();
        write!(f, "Actor[{}#{}]", path, uid)
    }
}

impl<T: ?Sized> ActorRefExt for T where T: TActorRef {}

pub trait ActorRefExt: TActorRef {
    fn tell_local<M>(&self, message: M, sender: Option<ActorRef>) where M: Message {
        let local = ActorMessage::local(message);
        self.tell(local, sender);
    }
    fn tell_remote<M>(&self, message: &M, sender: Option<ActorRef>) -> anyhow::Result<()> where M: Message + MessageID + Serialize + DeserializeOwned {
        let remote = ActorMessage::remote(message)?;
        self.tell(remote, sender);
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SerializedActorRef {
    pub address: Address,
    pub path: Vec<String>,
    pub uid: i32,
}

impl FromStr for SerializedActorRef {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let url = Url::parse(s).context(format!("invalid url {}", s))?;
        let scheme = url.scheme().to_string();
        let username = url.username().to_string();
        let host = url.domain().ok_or(anyhow!("no host found in url {}",s))?;
        let port = url.port().ok_or(anyhow!("no port found in url {}",s))?;
        let addr: SocketAddrV4 = format!("{}:{}", host, port).parse()?;
        let mut path = url.path().split("/").map(|s| s.to_string()).collect::<Vec<_>>();
        path.remove(0);
        let uid: i32 = url.fragment().unwrap_or("0").parse()?;
        let address = Address {
            protocol: scheme,
            system: username,
            addr,
        };
        let actor_ref = SerializedActorRef {
            address,
            path,
            uid,
        };
        Ok(actor_ref)
    }
}

impl Display for SerializedActorRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}#{}", self.address, self.path.join("/"), self.uid)
    }
}

impl Into<SerializedActorRef> for ActorRef {
    fn into(self) -> SerializedActorRef {
        let p = self.path();
        let mut path = VecDeque::new();
        path.push_front(p.name().clone());
        let mut parent = p.parent();
        loop {
            match &parent {
                ActorPath::RootActorPath(r) => {
                    path.push_front(r.name().clone());
                    break;
                }
                ActorPath::ChildActorPath(c) => {
                    path.push_front(c.name().clone());
                }
            }
            parent = parent.parent();
        }
        SerializedActorRef {
            address: p.address().clone(),
            path: path.into_iter().collect(),
            uid: p.uid(),
        }
    }
}

#[cfg(test)]
mod ref_test {
    use crate::actor_ref::SerializedActorRef;
    use crate::address::Address;

    #[test]
    fn test_serde() {
        let actor_ref = SerializedActorRef {
            address: Address {
                protocol: "tcp".to_string(),
                system: "game".to_string(),
                addr: "127.0.0.1:1122".parse().unwrap(),
            },
            path: vec!["a".to_string(), "b".to_string()],
            uid: 112132434,
        };
        let url = actor_ref.to_string();
        let de_actor_ref: SerializedActorRef = url.parse().unwrap();
        assert_eq!(actor_ref, de_actor_ref);
    }
}
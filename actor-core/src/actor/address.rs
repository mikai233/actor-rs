use std::collections::VecDeque;
use std::fmt::{Display, Formatter};
use std::net::{SocketAddr, SocketAddrV4};
use std::str::FromStr;

use imstr::ImString;
use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum Protocol {
    Tcp,
}

impl Display for Protocol {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Protocol::Tcp => {
                write!(f, "tcp")
            }
        }
    }
}

impl FromStr for Protocol {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "tcp" => Ok(Protocol::Tcp),
            _ => Err(anyhow::anyhow!("Unknown protocol: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct Address {
    pub protocol: Protocol,
    pub system: ImString,
    pub addr: Option<SocketAddr>,
}

impl Address {
    pub fn new(
        protocol: Protocol,
        system: impl Into<ImString>,
        addr: Option<SocketAddr>,
    ) -> Self {
        let protocol = protocol.into();
        let system = system.into();
        Self {
            protocol,
            system,
            addr,
        }
    }
}

impl Display for Address {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match &self.addr {
            None => {
                write!(f, "{}://{}", self.protocol, self.system)?;
            }
            Some(addr) => {
                write!(f, "{}://{}@{}", self.protocol, self.system, addr)?;
            }
        }
        Ok(())
    }
}

pub trait PathUtils {
    fn split(s: &str, fragment: Option<&str>) -> VecDeque<String> {
        let mut paths: VecDeque<String> = s.split('/').map(|s| s.to_string()).collect();
        if let Some(last) = paths.back_mut() {
            if let Some(fragment) = fragment {
                last.push_str("#");
                last.push_str(fragment);
            }
        }
        paths
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AddressFromURIString;

impl AddressFromURIString {
    fn parse_from_url(url: &url::Url) -> anyhow::Result<Address> {
        let scheme = url.scheme();
        let user_info = url.username();
        let mut address = Address::new(scheme.parse()?, user_info, None);
        if let Some(url::Host::Ipv4(addr)) = url.host() {
            if let Some(port) = url.port() {
                address.addr = Some(SocketAddrV4::new(addr, port).into());
            }
        }
        Ok(address)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ActorPathExtractor;

impl ActorPathExtractor {
    pub fn extract(s: &str) -> anyhow::Result<(Address, VecDeque<String>)> {
        let url = url::Url::parse(s)?;
        let address = AddressFromURIString::parse_from_url(&url)?;
        let mut paths = Self::split(url.path(), url.fragment());
        paths.pop_front();
        Ok((address, paths))
    }
}

impl PathUtils for ActorPathExtractor {}

use std::fmt::{Debug, Formatter};
use std::ops::{Deref, DerefMut};
use etcd_client::Client;

#[derive(Clone)]
pub struct EtcdClient(Client);

impl Deref for EtcdClient {
    type Target = Client;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for EtcdClient {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Debug for EtcdClient {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EtcdClient")
            .finish_non_exhaustive()
    }
}

impl From<Client> for EtcdClient {
    fn from(value: Client) -> Self {
        EtcdClient(value)
    }
}

impl Into<Client> for EtcdClient {
    fn into(self) -> Client {
        self.0
    }
}
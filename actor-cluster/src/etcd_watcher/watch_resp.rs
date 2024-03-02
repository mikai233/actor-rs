use std::ops::Deref;
use etcd_client::WatchResponse;
use actor_derive::OrphanEmptyCodec;

#[derive(Debug, OrphanEmptyCodec)]
pub struct WatchResp {
    pub key: String,
    pub resp: WatchResponse,
}

impl Deref for WatchResp {
    type Target = WatchResponse;

    fn deref(&self) -> &Self::Target {
        &self.resp
    }
}
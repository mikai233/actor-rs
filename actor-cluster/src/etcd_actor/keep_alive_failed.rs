use actor_derive::OrphanEmptyCodec;

#[derive(Debug, OrphanEmptyCodec)]
pub struct KeepAliveFailed {
    pub id: i64,
    pub error: Option<etcd_client::Error>,
}
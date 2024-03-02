use actor_derive::OrphanEmptyCodec;

#[derive(Debug, OrphanEmptyCodec)]
pub struct LeaseKeepAliveFailed(pub i64);
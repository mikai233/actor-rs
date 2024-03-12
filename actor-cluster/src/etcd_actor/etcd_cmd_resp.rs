use actor_derive::OrphanEmptyCodec;

use crate::etcd_actor::delete::DeleteResp;
use crate::etcd_actor::get::GetResp;
use crate::etcd_actor::keep_alive::KeepAliveFailed;
use crate::etcd_actor::lock::LockResp;
use crate::etcd_actor::put::PutResp;
use crate::etcd_actor::unlock::UnlockResp;
use crate::etcd_actor::unwatch::UnwatchResp;
use crate::etcd_actor::watch::WatchResp;

#[derive(Debug, OrphanEmptyCodec)]
pub enum EtcdCmdResp {
    PutResp(PutResp),
    GetResp(GetResp),
    DeleteResp(DeleteResp),
    LockResp(LockResp),
    UnlockResp(UnlockResp),
    WatchResp(WatchResp),
    UnwatchResp(UnwatchResp),
    KeepAliveFailed(KeepAliveFailed),
}
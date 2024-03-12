use async_trait::async_trait;

use actor_cluster::etcd_actor::etcd_cmd_resp::EtcdCmdResp;
use actor_core::actor::context::ActorContext;
use actor_core::Message;
use actor_derive::EmptyCodec;

use crate::shard_coordinator::ShardCoordinator;

#[derive(Debug, EmptyCodec)]
pub(super) struct EtcdResp(pub(super) EtcdCmdResp);

#[async_trait]
impl Message for EtcdResp {
    type A = ShardCoordinator;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        todo!()
    }
}
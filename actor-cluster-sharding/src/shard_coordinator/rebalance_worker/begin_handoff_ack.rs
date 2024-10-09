use std::any::type_name;

use anyhow::Context as _;
use async_trait::async_trait;
use bincode::{Decode, Encode};
use tracing::debug;

use actor_core::actor::context::{Context, ActorContext};
use actor_core::ext::option_ext::OptionExt;
use actor_core::Message;
use actor_core::MessageCodec;

use crate::shard_coordinator::rebalance_worker::RebalanceWorker;
use crate::shard_region::ShardId;

#[derive(Debug, Encode, Decode, MessageCodec)]
pub(crate) struct BeginHandoffAck {
    pub(crate) shard: ShardId,
}

#[async_trait]
impl Message for BeginHandoffAck {
    type A = RebalanceWorker;

    async fn handle(self: Box<Self>, context: &mut Context, actor: &mut Self::A) -> anyhow::Result<()> {
        let shard = self.shard;
        let sender = context.sender().into_result().context(type_name::<BeginHandoffAck>())?.clone();
        if actor.shard == shard {
            debug!("{}: BeginHandOffAck for shard [{}] received from [{}].", actor.type_name, actor.shard, sender);
            actor.acked(context, &sender);
        } else {
            debug!("{}: Ignore unknown BeginHandOffAck for shard [{}] received from [{}].", actor.type_name, actor.shard, sender);
        }
        Ok(())
    }
}
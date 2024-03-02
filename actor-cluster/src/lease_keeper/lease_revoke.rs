use async_trait::async_trait;
use tracing::debug;

use actor_core::actor::context::{ActorContext, Context};
use actor_core::ext::option_ext::OptionExt;
use actor_core::Message;
use actor_derive::EmptyCodec;

use crate::lease_keeper::EtcdLeaseKeeper;

#[derive(Debug, EmptyCodec)]
struct LeaseRevoke;

#[async_trait]
impl Message for LeaseRevoke {
    type A = EtcdLeaseKeeper;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let keeper = actor.keeper.as_result()?;
        actor.client.lease_revoke(keeper.id()).await?;
        debug!("{} {} lease revoke success", context.myself(), keeper.id());
        Ok(())
    }
}
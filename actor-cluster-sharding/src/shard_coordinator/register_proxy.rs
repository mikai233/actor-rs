use async_trait::async_trait;
use bincode::{Decode, Encode};
use tracing::debug;

use actor_core::actor::context::{ActorContext1, ActorContext};
use actor_core::actor_ref::{ActorRef, ActorRefExt};
use actor_core::CMessageCodec;
use actor_core::Message;

use crate::shard_coordinator::coordinator_state::CoordinatorState;
use crate::shard_coordinator::shard_region_proxy_terminated::ShardRegionProxyTerminated;
use crate::shard_coordinator::ShardCoordinator;
use crate::shard_coordinator::state_update::ShardState;
use crate::shard_region::register_ack::RegisterAck;

#[derive(Debug, Clone, Encode, Decode, CMessageCodec)]
pub(crate) struct RegisterProxy {
    pub(crate) shard_region_proxy: ActorRef,
}

#[async_trait]
impl Message for RegisterProxy {
    type A = ShardCoordinator;

    async fn handle(self: Box<Self>, context: &mut ActorContext1, actor: &mut Self::A) -> anyhow::Result<()> {
        let proxy = self.shard_region_proxy;
        if matches!(actor.coordinator_state, CoordinatorState::WaitingForStateInitialized) {
            debug!("{}: ShardRegion proxy tried to register bug ShardCoordinator not initialized yet: [{}]", actor.type_name, proxy);
            return Ok(());
        }
        if actor.is_member(context, &proxy) {
            debug!("{}: ShardRegion proxy registered: [{}]", actor.type_name, proxy);
            actor.inform_about_current_shards(&proxy);
            if actor.state.region_proxies.contains(&proxy) {
                proxy.cast_ns(RegisterAck { coordinator: context.myself().clone() });
            } else {
                actor.update_state(context, ShardState::ShardRegionProxyRegistered { region_proxy: proxy.clone() }).await;
                context.watch_with(proxy.clone(), ShardRegionProxyTerminated::new)?;
                proxy.cast_ns(RegisterAck { coordinator: context.myself().clone() });
            }
        }
        Ok(())
    }
}
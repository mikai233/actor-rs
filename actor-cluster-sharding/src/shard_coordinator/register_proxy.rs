use actor_core::actor::behavior::Behavior;
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::message::handler::MessageHandler;
use serde::{Deserialize, Serialize};
use tracing::debug;

use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor_ref::{ActorRef, ActorRefExt};
use actor_core::{Message, MessageCodec};

use crate::shard_coordinator::coordinator_state::CoordinatorState;
use crate::shard_coordinator::shard_region_proxy_terminated::ShardRegionProxyTerminated;
use crate::shard_coordinator::state_update::ShardState;
use crate::shard_coordinator::ShardCoordinator;
use crate::shard_region::register_ack::RegisterAck;

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    Message,
    MessageCodec,
    derive_more::Display,
    derive_more::Constructor,
)]
#[display("RegisterProxy {{ shard_region_proxy: {shard_region_proxy} }}")]
#[cloneable]
pub(crate) struct RegisterProxy {
    pub(crate) shard_region_proxy: ActorRef,
}

impl MessageHandler<ShardCoordinator> for RegisterProxy {
    fn handle(
        actor: &mut ShardCoordinator,
        ctx: &mut <ShardCoordinator as Actor>::Context,
        message: Self,
        sender: Option<ActorRef>,
        _: &Receive<ShardCoordinator>,
    ) -> anyhow::Result<Behavior<ShardCoordinator>> {
        let proxy = self.shard_region_proxy;
        if matches!(
            actor.coordinator_state,
            CoordinatorState::WaitingForStateInitialized
        ) {
            debug!("{}: ShardRegion proxy tried to register bug ShardCoordinator not initialized yet: [{}]", actor.type_name, proxy);
            return Ok(());
        }
        if actor.is_member(context, &proxy) {
            debug!(
                "{}: ShardRegion proxy registered: [{}]",
                actor.type_name, proxy
            );
            actor.inform_about_current_shards(&proxy);
            if actor.state.region_proxies.contains(&proxy) {
                proxy.cast_ns(RegisterAck {
                    coordinator: context.myself().clone(),
                });
            } else {
                actor
                    .update_state(
                        context,
                        ShardState::ShardRegionProxyRegistered {
                            region_proxy: proxy.clone(),
                        },
                    )
                    .await;
                context.watch_with(proxy.clone(), ShardRegionProxyTerminated::new)?;
                proxy.cast_ns(RegisterAck {
                    coordinator: context.myself().clone(),
                });
            }
        }
        Ok(Behavior::same())
    }
}

use std::collections::HashSet;
use std::ops::Not;
use std::time::Duration;

use async_trait::async_trait;
use tracing::{debug, warn};

use actor_core::actor::actor_ref::ActorRefExt;
use actor_core::actor::actor_ref_factory::ActorRefFactory;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::ext::option_ext::OptionExt;
use actor_core::Message;
use actor_derive::EmptyCodec;

use crate::handoff_stopper::HandoffStopper;
use crate::shard::handoff_stopper_terminated::HandoffStopperTerminated;
use crate::shard::Shard;
use crate::shard_coordinator::shard_stopped::ShardStopped;
use crate::shard_region::ShardId;

#[derive(Debug, EmptyCodec)]
pub(crate) struct Handoff {
    pub(crate) shard: ShardId,
}

#[async_trait]
impl Message for Handoff {
    type A = Shard;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let shard_id = self.shard;
        if shard_id == actor.shard_id {
            match &actor.handoff_stopper {
                None => {
                    debug!("{}: Handoff shard [{}]", actor.type_name, actor.shard_id);
                    let active_entities = actor.entities.active_entities();
                    if actor.preparing_for_shutdown {} else if active_entities.is_empty().not() && !actor.preparing_for_shutdown {
                        debug!("{}: Starting HandoffStopper for shard [{}] to terminate [{}] entities", actor.type_name, shard_id, active_entities.len());
                        for entity in &active_entities {
                            context.unwatch(entity);
                        }
                        let entities = active_entities.iter().map(|a| (**a).clone()).collect::<HashSet<_>>();
                        let reply_to = context.sender().into_result()?;
                        let stopper = context.spawn(
                            HandoffStopper::props(
                                actor.type_name.clone(),
                                actor.shard_id.clone(), reply_to.clone(),
                                entities,
                                actor.handoff_stop_message.dyn_clone()?,
                                Duration::from_secs(5),
                            ),
                            "handoff_stopper")?;
                        context.watch(HandoffStopperTerminated(stopper.clone()));
                        actor.handoff_stopper = Some(stopper);
                    } else {
                        let reply_to = context.sender().into_result()?;
                        reply_to.cast_ns(ShardStopped { shard: shard_id });
                    }
                }
                Some(_) => {
                    warn!("{}: Handoff shard [{}] received during existing handoff", actor.type_name, actor.shard_id);
                }
            }
        } else {
            let type_name = &actor.type_name;
            let self_shard_id = &actor.shard_id;
            warn!("{type_name}: Shard [{self_shard_id}] can not hand off for another Shard [{shard_id}]")
        }
        Ok(())
    }
}
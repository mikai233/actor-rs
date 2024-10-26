use std::any::type_name;
use std::ops::Not;
use std::time::Duration;

use actor_core::actor::behavior::Behavior;
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::message::handler::MessageHandler;
use ahash::HashSet;
use anyhow::anyhow;
use tracing::{debug, info, warn};

use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::actor_ref::{ActorRef, ActorRefExt};
use actor_core::Message;

use crate::handoff_stopper::HandoffStopper;
use crate::shard::handoff_stopper_terminated::HandoffStopperTerminated;
use crate::shard::Shard;
use crate::shard_coordinator::rebalance_worker::shard_stopped::ShardStopped;
use crate::shard_region::ShardId;

#[derive(Debug, Message, derive_more::Display, derive_more::Constructor)]
#[display("Handoff {{ shard: {shard} }}")]
pub(crate) struct Handoff {
    pub(crate) shard: ShardId,
}

impl MessageHandler<Shard> for Handoff {
    fn handle(
        actor: &mut Shard,
        ctx: &mut <Shard as Actor>::Context,
        message: Self,
        sender: Option<ActorRef>,
        _: &Receive<Shard>,
    ) -> anyhow::Result<Behavior<Shard>> {
        let shard_id = message.shard;
        if shard_id.as_str() == actor.shard_id.as_str() {
            match &actor.handoff_stopper {
                None => {
                    debug!("{}: Handoff shard [{}]", actor.type_name, actor.shard_id);
                    let active_entities = actor.entities.active_entities();
                    if actor.preparing_for_shutdown {
                        info!("{}: Handoff shard [{}] while preparing for shutdown. Stopping right away.", actor.type_name, shard_id);
                        for entity in active_entities {
                            entity.cast_ns(actor.handoff_stop_message.clone_box().unwrap());
                        }
                        let reply_to = sender.ok_or(anyhow!("Sender is None"))?;
                        reply_to.cast_ns(ShardStopped { shard: shard_id });
                        ctx.stop(ctx.myself());
                    } else if active_entities.is_empty().not() && !actor.preparing_for_shutdown {
                        debug!(
                            "{}: Starting HandoffStopper for shard [{}] to terminate [{}] entities",
                            actor.type_name,
                            shard_id,
                            active_entities.len()
                        );
                        for entity in &active_entities {
                            ctx.unwatch(entity);
                        }
                        let entities = active_entities
                            .iter()
                            .map(|a| (**a).clone())
                            .collect::<HashSet<_>>();
                        let reply_to = sender.ok_or(anyhow!("Sender is None"))?;
                        let stopper = ctx.spawn(
                            HandoffStopper::props(
                                actor.type_name.clone(),
                                actor.shard_id.clone(),
                                reply_to.clone(),
                                entities,
                                actor.handoff_stop_message.dyn_clone()?,
                                Duration::from_secs(5),
                            ),
                            "handoff_stopper",
                        )?;
                        ctx.watch(&stopper)?;
                        actor.handoff_stopper = Some(stopper);
                    } else {
                        let reply_to = sender.ok_or(anyhow!("Sender is None"))?;
                        reply_to.cast_ns(ShardStopped { shard: shard_id });
                        ctx.stop(ctx.myself());
                    }
                }
                Some(_) => {
                    warn!(
                        "{}: Handoff shard [{}] received during existing handoff",
                        actor.type_name, actor.shard_id
                    );
                }
            }
        } else {
            let type_name = &actor.type_name;
            let self_shard_id = &actor.shard_id;
            warn!("{type_name}: Shard [{self_shard_id}] can not hand off for another Shard [{shard_id}]")
        }
        Ok(Behavior::same())
    }
}

use crate::cluster_sharding::ClusterSharding;
use crate::cluster_sharding_guardian::started::Started;
use crate::cluster_sharding_guardian::ClusterShardingGuardian;
use crate::cluster_sharding_settings::ClusterShardingSettings;
use crate::message_extractor::MessageExtractor;
use crate::shard_region::ShardRegion;
use actor_core::actor::behavior::Behavior;
use actor_core::actor::context::ActorContext;
use actor_core::actor::receive::Receive;
use actor_core::actor::Actor;
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::actor_ref::{ActorRef, ActorRefExt};
use actor_core::message::handler::MessageHandler;
use actor_core::Message;
use anyhow::anyhow;
use imstr::ImString;
use std::sync::Arc;

#[derive(Debug, Message, derive_more::Display)]
#[display("StartProxy {{ type_name: {type_name}, settings: {settings}, message_extractor: {message_extractor} }}"
)]
pub(crate) struct StartProxy {
    pub(crate) type_name: ImString,
    pub(crate) settings: Arc<ClusterShardingSettings>,
    pub(crate) message_extractor: Box<dyn MessageExtractor>,
}

impl MessageHandler<ClusterShardingGuardian> for StartProxy {
    fn handle(
        _: &mut ClusterShardingGuardian,
        ctx: &mut <ClusterShardingGuardian as Actor>::Context,
        message: Self,
        sender: Option<ActorRef>,
        _: &Receive<ClusterShardingGuardian>,
    ) -> anyhow::Result<Behavior<ClusterShardingGuardian>> {
        let Self {
            type_name,
            settings,
            message_extractor,
        } = message;
        let enc_name = ClusterSharding::proxy_name(&type_name);
        let coordinator_path = ClusterShardingGuardian::coordinator_path(ctx.myself(), &type_name);
        let shard_region = match ctx.child(&enc_name) {
            None => ctx.spawn(
                ShardRegion::proxy_props(
                    enc_name.clone().into(),
                    settings,
                    coordinator_path,
                    message_extractor,
                ),
                enc_name,
            )?,
            Some(shard_region) => shard_region,
        };
        let started = Started::new(shard_region);
        let sender = sender.ok_or(anyhow!("Sender is None"))?;
        sender.cast_ns(started);
        Ok(Behavior::same())
    }
}
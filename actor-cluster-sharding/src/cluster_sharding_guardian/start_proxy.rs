use std::sync::Arc;

use async_trait::async_trait;
use imstr::ImString;

use actor_core::actor::context::{Context, ActorContext};
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::actor_ref::ActorRefExt;
use actor_core::EmptyCodec;
use actor_core::ext::option_ext::OptionExt;
use actor_core::Message;

use crate::cluster_sharding::ClusterSharding;
use crate::cluster_sharding_guardian::ClusterShardingGuardian;
use crate::cluster_sharding_guardian::started::Started;
use crate::cluster_sharding_settings::ClusterShardingSettings;
use crate::message_extractor::MessageExtractor;
use crate::shard_region::ShardRegion;

#[derive(Debug, EmptyCodec)]
pub(crate) struct StartProxy {
    pub(crate) type_name: ImString,
    pub(crate) settings: Arc<ClusterShardingSettings>,
    pub(crate) message_extractor: Box<dyn MessageExtractor>,
}

#[async_trait]
impl Message for StartProxy {
    type A = ClusterShardingGuardian;

    async fn handle(self: Box<Self>, context: &mut Context, _actor: &mut Self::A) -> anyhow::Result<()> {
        let Self { type_name, settings, message_extractor } = *self;
        let enc_name = ClusterSharding::proxy_name(&type_name);
        let coordinator_path = ClusterShardingGuardian::coordinator_path(context.myself(), &type_name);
        let shard_region = match context.child(&enc_name) {
            None => {
                context.spawn(
                    ShardRegion::proxy_props(
                        enc_name.clone().into(),
                        settings,
                        coordinator_path,
                        message_extractor,
                    ),
                    enc_name,
                )?
            }
            Some(shard_region) => { shard_region }
        };
        let started = Started { shard_region };
        context.sender().into_result()?.cast_orphan_ns(started);
        Ok(())
    }
}
use std::ops::{Deref, Not};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Context};
use dashmap::DashMap;
use dashmap::mapref::entry::Entry;
use dashmap::mapref::one::MappedRef;
use tracing::debug;

use actor_cluster::cluster::Cluster;
use actor_core::actor::actor_ref_factory::ActorRefFactory;
use actor_core::actor::actor_system::ActorSystem;
use actor_core::actor::extension::Extension;
use actor_core::actor::props::{Props, PropsBuilderSync};
use actor_core::actor_ref::{ActorRef, ActorRefExt};
use actor_core::config::Config;
use actor_core::DynMessage;
use actor_core::pattern::patterns::PatternsExt;
use actor_derive::AsAny;

use crate::{CLUSTER_SHARDING_CONFIG, CLUSTER_SHARDING_CONFIG_NAME};
use crate::cluster_sharding_guardian::ClusterShardingGuardian;
use crate::cluster_sharding_guardian::start::Start;
use crate::cluster_sharding_guardian::start_coordinator_if_needed::StartCoordinatorIfNeeded;
use crate::cluster_sharding_guardian::start_proxy::StartProxy;
use crate::cluster_sharding_guardian::started::Started;
use crate::cluster_sharding_settings::ClusterShardingSettings;
use crate::config::ClusterShardingConfig;
use crate::message_extractor::MessageExtractor;
use crate::shard_allocation_strategy::ShardAllocationStrategy;
use crate::shard_region::EntityId;

#[derive(Debug, Clone, AsAny)]
pub struct ClusterSharding {
    inner: Arc<Inner>,
}

#[derive(Debug)]
pub struct Inner {
    system: ActorSystem,
    cluster: Cluster,
    regions: DashMap<String, ActorRef>,
    proxies: DashMap<String, ActorRef>,
    guardian: ActorRef,
}

impl Deref for ClusterSharding {
    type Target = Arc<Inner>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl ClusterSharding {
    pub fn new(system: ActorSystem, config: ClusterShardingConfig) -> anyhow::Result<Self> {
        let default_config: ClusterShardingConfig = toml::from_str(CLUSTER_SHARDING_CONFIG).context(format!("failed to load {}", CLUSTER_SHARDING_CONFIG_NAME))?;
        let sharding_config = config.with_fallback(default_config);
        let guardian_name = sharding_config.guardian_name.clone();
        system.add_config(sharding_config)?;
        let guardian = system.spawn_system(Props::new_with_ctx(|context| {
            Ok(ClusterShardingGuardian { cluster: Cluster::get(context.system()).clone() })
        }), Some(guardian_name))?;
        let cluster = Cluster::get(&system).clone();
        let inner = Inner {
            system,
            cluster,
            regions: Default::default(),
            proxies: Default::default(),
            guardian,
        };
        Ok(ClusterSharding { inner: Arc::new(inner) })
    }

    pub fn get(system: &ActorSystem) -> MappedRef<&'static str, Box<dyn Extension>, Self> {
        system.get_extension::<Self>().expect("ClusterSharding extension not found")
    }

    pub async fn start<E, S>(
        &self,
        type_name: impl Into<String>,
        entity_props: PropsBuilderSync<EntityId>,
        settings: Arc<ClusterShardingSettings>,
        extractor: E,
        allocation_strategy: S,
        handoff_message: DynMessage,
    ) -> anyhow::Result<ActorRef> where
        E: MessageExtractor + 'static,
        S: ShardAllocationStrategy + 'static {
        let type_name = type_name.into();
        if handoff_message.is_cloneable().not() {
            let msg_name = handoff_message.name();
            return Err(anyhow!("entity {type_name} handoff message {msg_name} must be cloneable"));
        }
        if settings.should_host_shard(&self.cluster) {
            match self.regions.entry(type_name.clone()) {
                Entry::Occupied(o) => {
                    Ok(o.get().clone())
                }
                Entry::Vacant(v) => {
                    let start = Start {
                        type_name: v.key().to_string(),
                        entity_props,
                        settings,
                        message_extractor: Box::new(extractor),
                        allocation_strategy: Box::new(allocation_strategy),
                        handoff_stop_message: handoff_message,
                    };
                    let started: Started = self.guardian.ask(start, Duration::from_secs(3)).await?;
                    let shard_region = started.shard_region;
                    v.insert(shard_region.clone());
                    Ok(shard_region)
                }
            }
        } else {
            debug!("starting shard region proxy [{}] (no actors will be hosted on this node)...", type_name);
            if settings.should_host_coordinator(&self.cluster) {
                let start_coordinator_msg = StartCoordinatorIfNeeded {
                    type_name,
                    settings,
                    allocation_strategy: Box::new(allocation_strategy),
                };
                self.guardian.cast_ns(start_coordinator_msg);
            }
            todo!()
        }
    }

    pub async fn start_proxy<E>(
        &self,
        type_name: impl Into<String>,
        role: Option<String>,
        extractor: E,
    ) -> anyhow::Result<ActorRef> where
        E: MessageExtractor + 'static {
        let type_name = type_name.into();
        match self.proxies.entry(Self::proxy_name(&type_name)) {
            Entry::Occupied(o) => {
                Ok(o.get().clone())
            }
            Entry::Vacant(v) => {
                let mut settings = ClusterShardingSettings::create(&self.system);
                settings.role = role;
                let settings = Arc::new(settings);
                let start_msg = StartProxy {
                    type_name: type_name.clone(),
                    settings,
                    message_extractor: Box::new(extractor),
                };
                let started: Started = self.guardian.ask(start_msg, Duration::from_secs(3)).await?;
                let shard_region = started.shard_region;
                v.insert(shard_region.clone());
                Ok(shard_region)
            }
        }
    }

    pub(crate) fn proxy_name(type_name: &str) -> String {
        format!("{}_proxy", type_name)
    }
}
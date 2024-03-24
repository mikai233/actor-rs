use std::ops::{Deref, Not};
use std::sync::Arc;
use std::time::Duration;

use anyhow::anyhow;
use dashmap::DashMap;
use dashmap::mapref::entry::Entry;
use imstr::ImString;
use tracing::debug;

use actor_cluster::cluster::Cluster;
use actor_core::actor::actor_system::{ActorSystem, WeakActorSystem};
use actor_core::actor::extension::Extension;
use actor_core::actor::props::{Props, PropsBuilderSync};
use actor_core::actor_ref::{ActorRef, ActorRefExt};
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::DynMessage;
use actor_core::ext::type_name_of;
use actor_core::pattern::patterns::PatternsExt;
use actor_derive::AsAny;

use crate::cluster_sharding_guardian::ClusterShardingGuardian;
use crate::cluster_sharding_guardian::start::Start;
use crate::cluster_sharding_guardian::start_coordinator_if_needed::StartCoordinatorIfNeeded;
use crate::cluster_sharding_guardian::start_proxy::StartProxy;
use crate::cluster_sharding_guardian::started::Started;
use crate::cluster_sharding_settings::ClusterShardingSettings;
use crate::config::ClusterShardingConfig;
use crate::message_extractor::MessageExtractor;
use crate::shard_allocation_strategy::ShardAllocationStrategy;
use crate::shard_region::ImEntityId;

#[derive(Debug, Clone, AsAny)]
pub struct ClusterSharding {
    inner: Arc<Inner>,
}

#[derive(Debug)]
pub struct Inner {
    system: WeakActorSystem,
    cluster: Cluster,
    regions: DashMap<ImString, ActorRef>,
    proxies: DashMap<ImString, ActorRef>,
    guardian: ActorRef,
}

impl Deref for ClusterSharding {
    type Target = Arc<Inner>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Extension for ClusterSharding {}

impl ClusterSharding {
    pub fn new(system: ActorSystem, config: ClusterShardingConfig) -> anyhow::Result<Self> {
        // let default_config: ClusterShardingConfig = toml::from_str(CLUSTER_SHARDING_CONFIG).context(format!("failed to load {}", CLUSTER_SHARDING_CONFIG_NAME))?;
        // let sharding_config = config.with_fallback(default_config);
        //TODO
        let sharding_config = config;
        let guardian_name = sharding_config.guardian_name.clone();
        system.add_config(sharding_config)?;
        let guardian = system.spawn_system(Props::new_with_ctx(|context| {
            Ok(ClusterShardingGuardian { cluster: Cluster::get(context.system()).clone() })
        }), Some(guardian_name))?;
        let cluster = Cluster::get(&system).clone();
        let inner = Inner {
            system: system.downgrade(),
            cluster,
            regions: Default::default(),
            proxies: Default::default(),
            guardian,
        };
        Ok(ClusterSharding { inner: Arc::new(inner) })
    }

    pub fn get(system: &ActorSystem) -> Self {
        system.get_ext::<Self>().expect(&format!("{} not found", type_name_of::<Self>()))
    }

    pub async fn start<E, S>(
        &self,
        type_name: impl Into<String>,
        entity_props: PropsBuilderSync<ImEntityId>,
        settings: Arc<ClusterShardingSettings>,
        extractor: E,
        allocation_strategy: S,
        handoff_message: DynMessage,
    ) -> anyhow::Result<ActorRef> where
        E: MessageExtractor + 'static,
        S: ShardAllocationStrategy + 'static {
        let type_name: ImString = type_name.into().into();
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
                        type_name: v.key().clone(),
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
            let role = settings.role.clone();
            if settings.should_host_coordinator(&self.cluster) {
                let start_coordinator_msg = StartCoordinatorIfNeeded {
                    type_name: type_name.clone(),
                    settings,
                    allocation_strategy: Box::new(allocation_strategy),
                };
                self.guardian.cast_ns(start_coordinator_msg);
            }
            self.start_proxy(type_name, role, extractor).await
        }
    }

    pub async fn start_proxy<E>(
        &self,
        type_name: impl Into<String>,
        role: Option<String>,
        extractor: E,
    ) -> anyhow::Result<ActorRef> where
        E: MessageExtractor + 'static {
        let type_name: ImString = type_name.into().into();
        let proxy_name = Self::proxy_name(type_name.as_str());
        match self.proxies.get(proxy_name.as_str()) {
            None => {
                let mut settings = ClusterShardingSettings::create(&self.system.upgrade()?);
                settings.role = role;
                let settings = Arc::new(settings);
                let start_msg = StartProxy {
                    type_name: type_name.clone(),
                    settings,
                    message_extractor: Box::new(extractor),
                };
                let started: Started = self.guardian.ask(start_msg, Duration::from_secs(3)).await?;
                let shard_region = started.shard_region;
                self.proxies.insert(proxy_name.into(), shard_region.clone());
                Ok(shard_region)
            }
            Some(proxy) => Ok(proxy.value().clone()),
        }
    }

    pub(crate) fn proxy_name(type_name: &str) -> String {
        format!("{}_proxy", type_name)
    }
}
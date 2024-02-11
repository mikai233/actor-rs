use std::ops::Deref;
use std::sync::Arc;

use dashmap::DashMap;
use dashmap::mapref::one::MappedRef;

use actor_cluster::cluster::Cluster;
use actor_core::actor::actor_ref::ActorRef;
use actor_core::actor::actor_ref_factory::ActorRefFactory;
use actor_core::actor::actor_system::ActorSystem;
use actor_core::actor::extension::Extension;
use actor_core::actor::props::Props;
use actor_derive::AsAny;

use crate::cluster_sharding_guardian::ClusterShardingGuardian;

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
    pub fn new(system: ActorSystem) -> anyhow::Result<Self> {
        let cluster = Cluster::get(&system).clone();
        let guardian = system.spawn_system(Props::create(|context| {
            Ok(ClusterShardingGuardian {
                cluster: Cluster::get(context.system()).clone(),
            })
        }), Some("sharding".to_string()))?;
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
}
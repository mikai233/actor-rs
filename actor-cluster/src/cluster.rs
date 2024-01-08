use std::collections::HashSet;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;

use arc_swap::Guard;
use etcd_client::Client;

use actor_core::actor::actor_ref::ActorRef;
use actor_core::actor::actor_ref_factory::ActorRefFactory;
use actor_core::actor::actor_ref_provider::{ActorRefProvider, TActorRefProvider};
use actor_core::actor::actor_system::ActorSystem;
use actor_core::actor::address::Address;
use actor_core::actor::props::Props;
use actor_derive::AsAny;

use crate::cluster_daemon::ClusterDaemon;
use crate::cluster_provider::ClusterActorRefProvider;
use crate::unique_address::UniqueAddress;

#[derive(AsAny)]
pub struct Cluster {
    system: ActorSystem,
    eclient: Client,
    self_unique_address: UniqueAddress,
    roles: HashSet<String>,
    daemon: ActorRef,
}

impl Debug for Cluster {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_struct("Cluster")
            .field("system", &self.system)
            .field("eclient", &"..")
            .field("self_unique_address", &self.self_unique_address)
            .field("roles", &self.roles)
            .field("daemon", &self.daemon)
            .finish()
    }
}

impl Cluster {
    pub fn new(system: ActorSystem) -> Self {
        let provider = system.provider();
        let cluster_provider = Self::cluster_provider(&provider);
        let eclient = cluster_provider.eclient.clone();
        let roles = cluster_provider.roles.clone();
        let address = cluster_provider.get_default_address();
        let unique_address = UniqueAddress {
            address: address.clone(),
            uid: system.uid(),
        };
        let eclient_c = eclient.clone();
        let unique_address_c = unique_address.clone();
        let roles_c = roles.clone();
        let cluster_daemon = system.spawn_system_actor(Props::create(move |_| {
            ClusterDaemon {
                eclient: eclient_c.clone(),
                self_unique_address: unique_address_c.clone(),
                roles: roles_c.clone(),
            }
        }), Some("cluster".to_string()))
            .expect("Failed to create cluster daemon");
        Self {
            system,
            eclient,
            self_unique_address: unique_address,
            roles,
            daemon: cluster_daemon,
        }
    }

    pub fn cluster_provider(provider: &Guard<Arc<ActorRefProvider>>) -> &ClusterActorRefProvider {
        let cluster_provider =
            provider.as_any()
                .downcast_ref::<ClusterActorRefProvider>()
                .expect("expect ClusterActorRefProvider");
        cluster_provider
    }

    pub fn join(&self, address: Address) {}

    pub fn leave(&self, address: Address) {}

    pub fn self_roles(&self) -> &HashSet<String> {
        &self.roles
    }

    pub fn self_member(&self) {}

    pub fn self_address(&self) -> &Address {
        &self.self_unique_address.address
    }

    pub fn self_unique_address(&self) -> &UniqueAddress {
        &self.self_unique_address
    }
}
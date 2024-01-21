use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Formatter};
use std::ops::Deref;
use std::sync::{Arc, RwLockReadGuard, RwLockWriteGuard};

use arc_swap::Guard;
use dashmap::mapref::one::MappedRef;
use etcd_client::Client;

use actor_core::actor::actor_ref::ActorRef;
use actor_core::actor::actor_ref_factory::ActorRefFactory;
use actor_core::actor::actor_ref_provider::{ActorRefProvider, TActorRefProvider};
use actor_core::actor::actor_system::ActorSystem;
use actor_core::actor::address::Address;
use actor_core::actor::extension::Extension;
use actor_core::actor::props::Props;
use actor_core::DynMessage;
use actor_core::event::EventBus;
use actor_derive::AsAny;

use crate::cluster_daemon::ClusterDaemon;
use crate::cluster_event::ClusterEvent;
use crate::cluster_provider::ClusterActorRefProvider;
use crate::cluster_state::ClusterState;
use crate::member::{Member, MemberStatus};
use crate::unique_address::UniqueAddress;

#[derive(Clone, AsAny)]
pub struct Cluster {
    inner: Arc<ClusterInner>,
}

pub struct ClusterInner {
    system: ActorSystem,
    eclient: Client,
    self_unique_address: UniqueAddress,
    roles: HashSet<String>,
    daemon: ActorRef,
    state: ClusterState,
}

impl Deref for Cluster {
    type Target = ClusterInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
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
        let transport = cluster_provider.remote.transport.clone();
        let cluster_daemon = system.spawn_system(Props::create(move |_| {
            ClusterDaemon {
                eclient: eclient_c.clone(),
                self_addr: unique_address_c.clone(),
                roles: roles_c.clone(),
                transport: transport.clone(),
                lease_id: 0,
                key_addr: HashMap::new(),
                cluster: None,
            }
        }), Some("cluster".to_string()))
            .expect("Failed to create cluster daemon");
        let state = ClusterState::new(Member::new(unique_address.clone(), MemberStatus::Down, roles.clone()));
        let inner = ClusterInner {
            system,
            eclient,
            self_unique_address: unique_address,
            roles,
            daemon: cluster_daemon,
            state,
        };
        Self { inner: Arc::new(inner) }
    }

    pub fn get(system: &ActorSystem) -> MappedRef<&'static str, Box<dyn Extension>, Self> {
        system.get_extension::<Self>().expect("Cluster extension not found")
    }

    pub fn cluster_provider(provider: &Guard<Arc<ActorRefProvider>>) -> &ClusterActorRefProvider {
        let cluster_provider =
            provider.as_any()
                .downcast_ref::<ClusterActorRefProvider>()
                .expect("expect ClusterActorRefProvider");
        cluster_provider
    }

    pub fn subscribe_cluster_event(&self, subscriber: ActorRef) {
        let members = self.members().clone();
        let self_member = self.self_member().clone();
        subscriber.tell(DynMessage::orphan(ClusterEvent::current_cluster_state(members, self_member)), ActorRef::no_sender());
        self.system.event_stream().subscribe(subscriber, std::any::type_name::<ClusterEvent>());
    }

    pub fn unsubscribe_cluster_event(&self, subscriber: &ActorRef) {
        self.system.event_stream().unsubscribe(subscriber, std::any::type_name::<ClusterEvent>());
    }

    pub fn leave(&self, address: Address) {}

    pub fn self_roles(&self) -> &HashSet<String> {
        &self.roles
    }

    pub fn self_member(&self) -> RwLockReadGuard<Member> {
        self.state.self_member.read().unwrap()
    }

    pub(crate) fn self_member_write(&self) -> RwLockWriteGuard<Member> {
        self.state.self_member.write().unwrap()
    }

    pub fn self_address(&self) -> &Address {
        &self.self_unique_address.address
    }

    pub fn self_unique_address(&self) -> &UniqueAddress {
        &self.self_unique_address
    }

    pub(crate) fn state(&self) -> &ClusterState {
        &self.state
    }

    pub fn members(&self) -> RwLockReadGuard<HashMap<UniqueAddress, Member>> {
        self.state.members.read().unwrap()
    }

    pub(crate) fn members_write(&self) -> RwLockWriteGuard<HashMap<UniqueAddress, Member>> {
        self.state.members.write().unwrap()
    }

    pub fn cluster_leader(&self) -> Option<Member> {
        let members = self.members();
        let mut members = members.values().filter(|m| m.status == MemberStatus::Up).collect::<Vec<_>>();
        members.sort_by_key(|m| &m.addr);
        members.first().map(|leader| *leader).cloned()
    }

    pub fn role_leader(&self, role: &str) -> Option<Member> {
        let members = self.members();
        let mut role_members = members.values().filter(|m| m.has_role(role) && m.status == MemberStatus::Up).collect::<Vec<_>>();
        role_members.sort_by_key(|m| &m.addr);
        role_members.first().map(|leader| *leader).cloned()
    }
}
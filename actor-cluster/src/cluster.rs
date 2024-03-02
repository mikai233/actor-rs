use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::ops::Deref;
use std::sync::{Arc, RwLockReadGuard, RwLockWriteGuard};
use std::sync::atomic::{AtomicBool, Ordering};

use dashmap::mapref::one::MappedRef;

use actor_core::actor::actor_ref::{ActorRef, ActorRefExt};
use actor_core::actor::actor_ref_factory::ActorRefFactory;
use actor_core::actor::actor_ref_provider::{downcast_provider, TActorRefProvider};
use actor_core::actor::actor_system::ActorSystem;
use actor_core::actor::address::Address;
use actor_core::actor::extension::Extension;
use actor_core::actor::props::Props;
use actor_core::DynMessage;
use actor_core::event::EventBus;
use actor_core::ext::etcd_client::EtcdClient;
use actor_core::ext::type_name_of;
use actor_derive::AsAny;
use crate::cluster_daemon::add_on_member_removed_listener::AddOnMemberRemovedListener;
use crate::cluster_daemon::add_on_member_up_listener::AddOnMemberUpListener;

use crate::cluster_daemon::ClusterDaemon;
use crate::cluster_daemon::leave_cluster::LeaveCluster;
use crate::cluster_event::ClusterEvent;
use crate::cluster_provider::ClusterActorRefProvider;
use crate::cluster_state::ClusterState;
use crate::member::{Member, MemberStatus};
use crate::unique_address::UniqueAddress;

#[derive(Clone, Debug, AsAny)]
pub struct Cluster {
    inner: Arc<Inner>,
}

#[derive(Debug)]
pub struct Inner {
    system: ActorSystem,
    client: EtcdClient,
    self_unique_address: UniqueAddress,
    roles: HashSet<String>,
    daemon: ActorRef,
    state: ClusterState,
    is_terminated: AtomicBool,
}

impl Deref for Cluster {
    type Target = Inner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Cluster {
    pub fn new(system: ActorSystem) -> anyhow::Result<Self> {
        let provider = system.provider();
        let cluster_provider = downcast_provider::<ClusterActorRefProvider>(&provider);
        let client = cluster_provider.client.clone();
        let roles = cluster_provider.roles.clone();
        let address = cluster_provider.get_default_address();
        let self_unique_address = UniqueAddress {
            address: address.clone(),
            uid: system.uid(),
        };
        let client_clone = client.clone();
        let unique_address_c = self_unique_address.clone();
        let roles_c = roles.clone();
        let transport = cluster_provider.remote.transport.clone();
        let daemon = system.spawn_system(Props::new_with_ctx(move |_| {
            Ok(ClusterDaemon {
                client: client_clone.clone(),
                self_addr: unique_address_c.clone(),
                roles: roles_c.clone(),
                transport: transport.clone(),
                key_addr: HashMap::new(),
                cluster: None,
            })
        }), Some("cluster".to_string()))?;
        let state = ClusterState::new(
            Member::new(self_unique_address.clone(), MemberStatus::Down, roles.clone(), 0)
        );
        let inner = Inner {
            system,
            client,
            self_unique_address,
            roles,
            daemon,
            state,
            is_terminated: AtomicBool::new(false),
        };
        Ok(Self { inner: Arc::new(inner) })
    }

    pub fn get(system: &ActorSystem) -> MappedRef<&'static str, Box<dyn Extension>, Self> {
        system.get_extension::<Self>().expect("Cluster extension not found")
    }

    pub fn subscribe_cluster_event(&self, subscriber: ActorRef) {
        let members = self.members().clone();
        let self_member = self.self_member().clone();
        subscriber.tell(DynMessage::orphan(ClusterEvent::current_cluster_state(members, self_member)), ActorRef::no_sender());
        self.system.event_stream().subscribe(subscriber, type_name_of::<ClusterEvent>());
    }

    pub fn unsubscribe_cluster_event(&self, subscriber: &ActorRef) {
        self.system.event_stream().unsubscribe(subscriber, type_name_of::<ClusterEvent>());
    }

    pub fn leave(&self, address: Address) {
        self.daemon.cast_ns(LeaveCluster(address));
    }

    pub fn register_on_member_up<F>(&self, f: F) where F: FnOnce() + Send + 'static {
        self.daemon.cast_ns(AddOnMemberUpListener(Box::new(f)));
    }

    pub fn register_on_member_removed<F>(&self, f: F) where F: FnOnce() + Send + 'static {
        self.daemon.cast_ns(AddOnMemberRemovedListener(Box::new(f)));
    }

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

    pub(crate) fn shutdown(&self) {
        if !self.is_terminated.swap(true, Ordering::Relaxed) {
            self.system.stop(&self.daemon);
        }
    }

    pub fn is_terminated(&self) -> bool {
        self.is_terminated.load(Ordering::Relaxed)
    }

    pub fn client(&self) -> &EtcdClient {
        &self.client
    }
}
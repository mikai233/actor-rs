use std::any::type_name;
use std::fmt::Debug;
use std::ops::Deref;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use ahash::{HashMap, HashSet};
use anyhow::{anyhow, Context};
use parking_lot::{Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};

use actor_core::actor::actor_system::{ActorSystem, WeakSystem};
use actor_core::actor::address::Address;
use actor_core::actor::extension::Extension;
use actor_core::actor::props::{ActorDeferredSpawn, DeferredSpawn, Props};
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::actor_ref::{ActorRef, ActorRefExt};
use actor_core::ext::option_ext::OptionExt;
use actor_core::pattern::patterns::PatternsExt;
use actor_core::provider::local_provider::LocalActorRefProvider;
use actor_core::provider::TActorRefProvider;
use actor_core::{AsAny, DynMessage};

use crate::cluster_core_daemon::leave::Leave;
use crate::cluster_daemon::add_on_member_removed_listener::AddOnMemberRemovedListener;
use crate::cluster_daemon::add_on_member_up_listener::AddOnMemberUpListener;
use crate::cluster_daemon::get_cluster_core_ref_req::{
    GetClusterCoreRefReq, GetClusterCoreRefResp,
};
use crate::cluster_daemon::internal_cluster_action::{Subscribe, Unsubscribe};
use crate::cluster_daemon::ClusterDaemon;
use crate::cluster_event::{ClusterDomainEvent, SubscriptionInitialStateMode};
use crate::cluster_provider::ClusterActorRefProvider;
use crate::cluster_settings::ClusterSettings;
use crate::cluster_state::ClusterState;
use crate::member::{Member, MemberStatus};
use crate::unique_address::UniqueAddress;

#[derive(Clone, Debug, AsAny)]
pub struct Cluster {
    inner: Arc<Inner>,
}

#[derive(Debug)]
pub struct Inner {
    pub settings: ClusterSettings,
    pub system: WeakSystem,
    pub self_unique_address: UniqueAddress,
    roles: HashSet<String>,
    pub cluster_daemons: ActorRef,
    cluster_core: RwLock<Option<ActorRef>>,
    state: ClusterState,
    is_terminated: AtomicBool,
    cluster_daemons_spawn: Mutex<Option<ActorDeferredSpawn>>,
}

impl Deref for Cluster {
    type Target = Inner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Extension for Cluster {
    fn init(&self) -> anyhow::Result<()> {
        let actor_spawn = self
            .cluster_daemons_spawn
            .lock()
            .take()
            .into_result()
            .context("cannot init Cluster more than once")?;
        let system = self.system()?;
        let provider = system.provider();
        let local = provider
            .downcast_ref::<LocalActorRefProvider>()
            .ok_or(anyhow!("LocalActorRefProvider not found"))?;
        let creation_timeout = local.settings().actor.creation_timeout.to_std_duration();
        Box::new(actor_spawn).spawn(system)?;
        let cluster_core = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                self.cluster_daemons
                    .ask::<_, GetClusterCoreRefResp>(GetClusterCoreRefReq, creation_timeout)
                    .await
            })
        })?
        .0;
        *self.cluster_core.write() = Some(cluster_core);
        Ok(())
    }
}

impl Cluster {
    pub fn new(system: ActorSystem) -> anyhow::Result<Self> {
        let provider = system.provider();
        let cluster_provider = provider
            .downcast_ref::<ClusterActorRefProvider>()
            .ok_or(anyhow!("ClusterActorRefProvider not found"))?;
        let roles = cluster_provider.roles.clone();
        let address = cluster_provider.get_default_address();
        let self_unique_address = UniqueAddress {
            address: address.clone(),
            uid: system.uid,
        };
        let (cluster_daemons, cluster_daemons_deferred) = system.spawn_system_deferred(
            Props::new_with_ctx(move |ctx| ClusterDaemon::new(ctx)),
            Some("cluster".to_string()),
        )?;
        let state = ClusterState::new(Member::new(
            self_unique_address.clone(),
            MemberStatus::Removed,
            roles.clone(),
            0,
        ));
        let inner = Inner {
            system: system.downgrade(),
            self_unique_address,
            roles,
            cluster_daemons,
            cluster_core: RwLock::new(None),
            state,
            is_terminated: AtomicBool::new(false),
            cluster_daemons_spawn: Mutex::new(Some(cluster_daemons_deferred)),
        };
        Ok(Self {
            inner: Arc::new(inner),
        })
    }

    pub fn get(system: &ActorSystem) -> Self {
        system
            .get_extension::<Self>()
            .expect(&format!("{} not found", type_name::<Self>()))
    }

    pub fn subscribe<E, T>(
        &self,
        subscriber: ActorRef,
        initial_state_mode: SubscriptionInitialStateMode,
        transform: T,
    ) where
        E: ClusterDomainEvent,
        T: Fn(Box<dyn ClusterDomainEvent>) -> DynMessage + Send + 'static,
    {
        let subscribe = Subscribe {
            subscriber,
            initial_state_mode,
            to: type_name::<E>(),
            transform,
        };
        self.cluster_core().cast_ns(subscribe);
    }

    pub fn subscribe_with_initial_state_as_snapshot<T>(&self, subscriber: ActorRef)
    where
        T: ClusterDomainEvent,
    {
        self.subscribe::<T>(
            subscriber,
            SubscriptionInitialStateMode::InitialStateAsSnapshot,
        );
    }

    pub fn unsubscribe<T>(&self, subscriber: ActorRef)
    where
        T: ClusterDomainEvent,
    {
        let unsubscribe = Unsubscribe {
            subscriber,
            to: Some(type_name::<T>()),
        };
        self.cluster_core().cast_ns(unsubscribe);
    }

    pub fn unsubscribe_all(&self, subscriber: ActorRef) {
        let unsubscribe = Unsubscribe {
            subscriber,
            to: None,
        };
        self.cluster_core().cast_ns(unsubscribe);
    }

    pub fn leave(&self, address: Address) {
        self.cluster_core().cast_ns(Leave(address));
    }

    pub fn register_on_member_up<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.cluster_daemons
            .cast_ns(AddOnMemberUpListener(Box::new(f)));
    }

    pub fn register_on_member_removed<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.cluster_daemons
            .cast_ns(AddOnMemberRemovedListener(Box::new(f)));
    }

    pub fn self_roles(&self) -> &HashSet<String> {
        &self.roles
    }

    pub fn self_member(&self) -> RwLockReadGuard<Member> {
        self.state.self_member.read()
    }

    pub(crate) fn self_member_write(&self) -> RwLockWriteGuard<Member> {
        self.state.self_member.write()
    }

    pub fn self_address(&self) -> &Address {
        &self.self_unique_address.address
    }

    pub fn self_unique_address(&self) -> &UniqueAddress {
        &self.self_unique_address
    }

    pub fn state(&self) -> &ClusterState {
        &self.state
    }

    pub fn members(&self) -> RwLockReadGuard<HashMap<UniqueAddress, Member>> {
        self.state().members()
    }

    pub(crate) fn members_write(&self) -> RwLockWriteGuard<HashMap<UniqueAddress, Member>> {
        self.state.members.write()
    }

    pub fn cluster_leader(&self) -> Option<Member> {
        let members = self.members();
        let mut members = members
            .values()
            .filter(|m| m.status == MemberStatus::Up)
            .collect::<Vec<_>>();
        members.sort_by_key(|m| &m.unique_address);
        members.first().map(|leader| *leader).cloned()
    }

    pub fn role_leader(&self, role: &str) -> Option<Member> {
        let members = self.members();
        let mut role_members = members
            .values()
            .filter(|m| m.has_role(role) && m.status == MemberStatus::Up)
            .collect::<Vec<_>>();
        role_members.sort_by_key(|m| &m.unique_address);
        role_members.first().map(|leader| *leader).cloned()
    }

    pub(crate) fn shutdown(&self) -> anyhow::Result<()> {
        if !self.is_terminated.swap(true, Ordering::Relaxed) {
            self.system.upgrade()?.stop(&self.cluster_daemons);
        }
        Ok(())
    }

    pub fn is_terminated(&self) -> bool {
        self.is_terminated.load(Ordering::Relaxed)
    }

    pub fn system(&self) -> anyhow::Result<ActorSystem> {
        self.system.upgrade()
    }

    pub fn cluster_core(&self) -> ActorRef {
        self.cluster_core.read().as_ref().unwrap();
        self.cluster_core
            .read()
            .clone()
            .expect("cluster_core not initialized")
    }
}

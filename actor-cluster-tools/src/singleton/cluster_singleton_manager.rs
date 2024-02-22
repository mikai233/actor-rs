use std::fmt::Debug;
use std::sync::Mutex;
use std::time::Duration;

use anyhow::anyhow;
use async_trait::async_trait;
use etcd_client::LockOptions;
use serde::{Deserialize, Serialize};
use tokio::task::AbortHandle;
use tracing::{debug, error, info, trace, warn};
use typed_builder::TypedBuilder;

use actor_cluster::cluster::Cluster;
use actor_cluster::cluster_provider::ClusterActorRefProvider;
use actor_cluster::lease_keeper::{EtcdLeaseKeeper, LeaseKeepAliveFailed};
use actor_cluster::member::MemberStatus;
use actor_core::{Actor, DynMessage, Message};
use actor_core::actor::actor_path::TActorPath;
use actor_core::actor::actor_ref::{ActorRef, ActorRefExt};
use actor_core::actor::actor_ref_factory::ActorRefFactory;
use actor_core::actor::actor_ref_provider::downcast_provider;
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::coordinated_shutdown::{CoordinatedShutdown, PHASE_CLUSTER_EXITING};
use actor_core::actor::props::Props;
use actor_core::ext::etcd_client::EtcdClient;
use actor_core::message::terminated::Terminated;
use actor_derive::EmptyCodec;

#[derive(Debug, Clone, Serialize, Deserialize, TypedBuilder)]
pub struct ClusterSingletonManagerSettings {
    #[builder(default = "singleton".to_string())]
    pub singleton_name: String,
    #[builder(default = None)]
    pub role: Option<String>,
}

impl Default for ClusterSingletonManagerSettings {
    fn default() -> Self {
        Self {
            singleton_name: "singleton".to_string(),
            role: None,
        }
    }
}

#[derive(Debug)]
pub struct ClusterSingletonManager {
    cluster: Cluster,
    singleton_props: Props,
    termination_message: DynMessage,
    settings: ClusterSingletonManagerSettings,
    client: EtcdClient,
    lease_id: i64,
    lock_key: Option<Vec<u8>>,
    lock_handle: Option<AbortHandle>,
    singleton: Option<ActorRef>,
    singleton_shutdown_notifier: Option<tokio::sync::oneshot::Sender<()>>,
}

impl ClusterSingletonManager {
    pub fn new(
        context: &mut ActorContext,
        props: Props,
        termination_message: DynMessage,
        settings: ClusterSingletonManagerSettings,
    ) -> anyhow::Result<Self> {
        let cluster = Cluster::get(context.system()).clone();
        let myself = context.myself().clone();
        let mut coordinate_shutdown = CoordinatedShutdown::get_mut(context.system());
        coordinate_shutdown.add_task(PHASE_CLUSTER_EXITING, "wait-singleton-exiting", async move {
            if !(cluster.is_terminated() || cluster.self_member().status == MemberStatus::Down) {
                let (tx, rx) = tokio::sync::oneshot::channel();
                myself.cast_ns(ShutdownSingleton(tx));
                let _ = rx.await;
            }
        })?;
        let cluster = Cluster::get(context.system()).clone();
        let provider = context.system().provider();
        let cluster_provider = downcast_provider::<ClusterActorRefProvider>(&provider);
        let client = cluster_provider.client.clone();
        let myself = Self {
            cluster,
            singleton_props: props,
            termination_message,
            settings,
            client,
            lease_id: 0,
            lock_key: None,
            lock_handle: None,
            singleton: None,
            singleton_shutdown_notifier: None,
        };
        Ok(myself)
    }

    fn singleton_name(&self) -> &String {
        &self.settings.singleton_name
    }

    async fn spawn_lease_keeper(&mut self, context: &mut ActorContext) -> anyhow::Result<i64> {
        let resp = self.client.lease_grant(30, None).await?;
        let lease_id = resp.id();
        let client = self.client.clone();
        let receiver = context.message_adapter::<LeaseKeepAliveFailed>(|_| DynMessage::user(LeaseFailed));
        context.spawn(
            Props::create(move |_| { Ok(EtcdLeaseKeeper::new(client.clone(), resp.id(), receiver.clone(), Duration::from_secs(3))) }),
            "lease_keeper",
        )?;
        Ok(lease_id)
    }

    async fn unlock(&mut self) -> anyhow::Result<()> {
        if let Some(lock_key) = self.lock_key.take() {
            self.client.unlock(lock_key).await?;
        }
        Ok(())
    }

    fn lock(&mut self, context: &mut ActorContext) {
        if let Some(role) = &self.settings.role {
            let self_member = self.cluster.self_member();
            if !self_member.has_role(role) {
                let addr = &self_member.addr;
                let name = context.myself().path().name();
                trace!("{} do not has role {}, no need to start singleton {}", addr, role, name);
                return;
            }
        }
        let myself = context.myself().clone();
        let system_name = context.system().name();
        let singleton_name = context.myself().path().name();
        let lock_path = singleton_path(system_name, singleton_name);
        let lock_options = LockOptions::new().with_lease(self.lease_id);
        let mut client = self.client.clone();
        let handle = context.spawn_fut(async move {
            match client.lock(lock_path.clone(), Some(lock_options)).await {
                Ok(resp) => {
                    myself.cast_ns(LockSuccess(resp.key().to_vec()));
                }
                Err(err) => {
                    let lock_failed = LockFailed {
                        path: lock_path,
                        error: err,
                    };
                    myself.cast_ns(lock_failed);
                }
            }
        });
        self.lock_handle = Some(handle);
    }

    pub fn props(props: Props, termination_message: DynMessage, settings: ClusterSingletonManagerSettings) -> anyhow::Result<Props> {
        if !termination_message.is_cloneable() {
            return Err(anyhow!("termination message {} require cloneable", termination_message.name()));
        }
        let termination_message = Mutex::new(termination_message);
        let props = Props::create(move |context| {
            let termination_message = termination_message.lock().unwrap().dyn_clone().unwrap();
            Self::new(context, props.clone(), termination_message, settings.clone())
        });
        Ok(props)
    }
}

fn singleton_path(system_name: &str, name: &str) -> String {
    format!("actor/{}/cluster/singleton/{}", system_name, name)
}

#[async_trait]
impl Actor for ClusterSingletonManager {
    async fn started(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        let lease_id = self.spawn_lease_keeper(context).await?;
        self.lease_id = lease_id;
        self.lock(context);
        Ok(())
    }

    async fn stopped(&mut self, _context: &mut ActorContext) -> anyhow::Result<()> {
        self.unlock().await?;
        Ok(())
    }
}

#[derive(Debug, EmptyCodec)]
struct LeaseFailed;

#[async_trait]
impl Message for LeaseFailed {
    type A = ClusterSingletonManager;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        match actor.spawn_lease_keeper(context).await {
            Ok(lease_id) => {
                actor.lease_id = lease_id;
                if let Some(handle) = actor.lock_handle.take() {
                    handle.abort();
                }
                actor.lock(context);
            }
            Err(error) => {
                let myself = context.myself().clone();
                let name = myself.path().name();
                let retry = Duration::from_secs(1);
                warn!("{} lease failed {:?}, retry after {:?}", name, error, retry);
                context.system().scheduler().schedule_once(retry, move || {
                    myself.cast_ns(LeaseFailed);
                });
            }
        }
        Ok(())
    }
}

#[derive(Debug, EmptyCodec)]
struct LockSuccess(Vec<u8>);

#[async_trait]
impl Message for LockSuccess {
    type A = ClusterSingletonManager;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        actor.lock_key = Some(self.0);
        match context.spawn(actor.singleton_props.clone(), actor.singleton_name()) {
            Ok(singleton) => {
                context.watch(SingletonTerminated(singleton.clone()));
                info!("singleton manager start singleton actor {}", singleton);
                actor.singleton = Some(singleton);
            }
            Err(error) => {
                let name = context.myself().path().name();
                error!("spawn singleton {} error {:?}", name, error);
                actor.unlock().await?;
            }
        };
        Ok(())
    }
}

#[derive(Debug, EmptyCodec)]
struct LockFailed {
    path: String,
    error: etcd_client::Error,
}

#[async_trait]
impl Message for LockFailed {
    type A = ClusterSingletonManager;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        let Self { path, error } = *self;
        error!("lock singleton {} failed {:?}, retry it", path, error);
        actor.lock(context);
        Ok(())
    }
}

#[derive(Debug, EmptyCodec)]
struct SingletonTerminated(ActorRef);

#[async_trait]
impl Message for SingletonTerminated {
    type A = ClusterSingletonManager;

    async fn handle(self: Box<Self>, _context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        debug!("singleton manager watch singleton actor {} terminated", self.0);
        actor.singleton = None;
        if let Some(notifier) = actor.singleton_shutdown_notifier.take() {
            let _ = notifier.send(());
        }
        actor.unlock().await?;
        Ok(())
    }
}

impl Terminated for SingletonTerminated {
    fn actor(&self) -> &ActorRef {
        &self.0
    }
}

#[derive(Debug, EmptyCodec)]
struct ShutdownSingleton(tokio::sync::oneshot::Sender<()>);

#[async_trait]
impl Message for ShutdownSingleton {
    type A = ClusterSingletonManager;

    async fn handle(self: Box<Self>, _context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        if let Some(singleton) = &actor.singleton {
            actor.singleton_shutdown_notifier = Some(self.0);
            singleton.tell(actor.termination_message.dyn_clone().unwrap(), ActorRef::no_sender());
        } else {
            let _ = self.0.send(());
        }
        Ok(())
    }
}
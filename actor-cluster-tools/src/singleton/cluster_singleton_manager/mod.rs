use std::fmt::Debug;
use std::time::Duration;

use anyhow::anyhow;
use async_trait::async_trait;
use etcd_client::LockOptions;
use serde::{Deserialize, Serialize};
use tokio::task::AbortHandle;
use tracing::trace;
use typed_builder::TypedBuilder;

use actor_cluster::cluster::Cluster;
use actor_cluster::cluster_provider::ClusterActorRefProvider;
use actor_cluster::etcd_actor::keep_alive::KeepAlive;
use actor_cluster::member::MemberStatus;
use actor_core::{Actor, DynMessage};
use actor_core::actor::context::{ActorContext, Context};
use actor_core::actor::coordinated_shutdown::{CoordinatedShutdown, PHASE_CLUSTER_EXITING};
use actor_core::actor::props::{Props, PropsBuilder};
use actor_core::actor_path::TActorPath;
use actor_core::actor_ref::{ActorRef, ActorRefExt};
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::ext::etcd_client::EtcdClient;
use actor_core::ext::message_ext::UserMessageExt;
use actor_core::provider::downcast_provider;

use crate::singleton::cluster_singleton_manager::lock_failed::LockFailed;
use crate::singleton::cluster_singleton_manager::lock_success::LockSuccess;
use crate::singleton::cluster_singleton_manager::shutdown_singleton::ShutdownSingleton;
use crate::singleton::cluster_singleton_manager::singleton_keep_alive_failed::SingletonKeepAliveFailed;

mod shutdown_singleton;
mod singleton_terminated;
mod lock_failed;
mod lock_success;
mod singleton_keep_alive_failed;

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
    singleton_props: PropsBuilder<()>,
    termination_message: DynMessage,
    settings: ClusterSingletonManagerSettings,
    client: EtcdClient,
    lease_id: i64,
    lock_key: Option<Vec<u8>>,
    lock_handle: Option<AbortHandle>,
    singleton: Option<ActorRef>,
    singleton_shutdown_notifier: Option<tokio::sync::oneshot::Sender<()>>,
    singleton_keep_alive_adapter: ActorRef,
}

impl ClusterSingletonManager {
    pub fn new(
        context: &mut ActorContext,
        props: PropsBuilder<()>,
        termination_message: DynMessage,
        settings: ClusterSingletonManagerSettings,
    ) -> anyhow::Result<Self> {
        let cluster = Cluster::get(context.system()).clone();
        let singleton_keep_alive_adapter = context.adapter(|m| { SingletonKeepAliveFailed(Some(m)).into_dyn() });
        let myself = context.myself().clone();
        let mut coordinate_shutdown = CoordinatedShutdown::get_mut(context.system());
        coordinate_shutdown.add_task(PHASE_CLUSTER_EXITING, "wait-singleton-exiting", async move {
            if !(cluster.is_terminated() || cluster.self_member().status == MemberStatus::Removed) {
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
            singleton_keep_alive_adapter,
        };
        Ok(myself)
    }

    fn singleton_name(&self) -> &String {
        &self.settings.singleton_name
    }

    async fn keep_alive(&mut self) -> anyhow::Result<i64> {
        let resp = self.client.lease_grant(30, None).await?;
        let lease_id = resp.id();
        let keep_alive = KeepAlive {
            id: lease_id,
            applicant: self.singleton_keep_alive_adapter.clone(),
            interval: Duration::from_secs(3),
        };
        self.cluster.etcd_actor().cast_ns(keep_alive);
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

    pub fn props(props: PropsBuilder<()>, termination_message: DynMessage, settings: ClusterSingletonManagerSettings) -> anyhow::Result<Props> {
        if !termination_message.is_cloneable() {
            return Err(anyhow!("termination message {} require cloneable", termination_message.name()));
        }
        let props = Props::new_with_ctx(move |context| {
            Self::new(context, props, termination_message, settings)
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
        let lease_id = self.keep_alive().await?;
        self.lease_id = lease_id;
        self.lock(context);
        Ok(())
    }

    async fn stopped(&mut self, _context: &mut ActorContext) -> anyhow::Result<()> {
        self.unlock().await?;
        Ok(())
    }
}
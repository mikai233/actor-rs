use std::fmt::Debug;
use std::time::Duration;

use anyhow::anyhow;
use async_trait::async_trait;
use typed_builder::TypedBuilder;

use actor_cluster::cluster::Cluster;
use actor_cluster::member::MemberStatus;
use actor_core::{Actor, CodecMessage, DynMessage};
use actor_core::actor::context::{Context, ActorContext};
use actor_core::actor::coordinated_shutdown::{CoordinatedShutdown, PHASE_CLUSTER_EXITING};
use actor_core::actor::props::{Props, PropsBuilder};
use actor_core::actor_path::TActorPath;
use actor_core::actor_ref::{ActorRef, ActorRefExt};
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;

use crate::config::singleton_config::SingletonConfig;
use crate::singleton::cluster_singleton_manager::shutdown_singleton::ShutdownSingleton;
use crate::singleton::cluster_singleton_manager::singleton_keep_alive_failed::SingletonKeepAliveFailed;

mod shutdown_singleton;
mod singleton_terminated;
mod lock_failed;
mod lock_success;
mod singleton_keep_alive_failed;

const SINGLETON_KEEP_ALIVE_INTERVAL: Duration = Duration::from_secs(3);
const SINGLETON_LEASE_TTL: i64 = 30;


#[derive(Debug, Clone, TypedBuilder)]
pub struct ClusterSingletonManagerSettings {
    #[builder(default = "singleton".to_string())]
    pub singleton_name: String,
    #[builder(default = None)]
    pub role: Option<String>,
}

impl From<SingletonConfig> for ClusterSingletonManagerSettings {
    fn from(value: SingletonConfig) -> Self {
        let SingletonConfig { singleton_name, role } = value;
        Self {
            singleton_name,
            role,
        }
    }
}

#[derive(Debug)]
pub struct ClusterSingletonManager {
    cluster: Cluster,
    singleton_props: PropsBuilder<()>,
    termination_message: DynMessage,
    settings: ClusterSingletonManagerSettings,
    singleton: Option<ActorRef>,
    singleton_shutdown_notifier: Option<tokio::sync::oneshot::Sender<()>>,
    singleton_keep_alive_adapter: ActorRef,
}

impl ClusterSingletonManager {
    pub fn new(
        context: &mut Context,
        props: PropsBuilder<()>,
        termination_message: DynMessage,
        settings: ClusterSingletonManagerSettings,
    ) -> anyhow::Result<Self> {
        let cluster = Cluster::get(context.system()).clone();
        let singleton_keep_alive_adapter = context.adapter(|m| { SingletonKeepAliveFailed(Some(m)).into_dyn() });
        let myself = context.myself().clone();
        let coordinate_shutdown = CoordinatedShutdown::get(context.system());
        coordinate_shutdown.add_task(context.system(), PHASE_CLUSTER_EXITING, "wait-singleton-exiting", async move {
            if !(cluster.is_terminated() || cluster.self_member().status == MemberStatus::Removed) {
                let (tx, rx) = tokio::sync::oneshot::channel();
                myself.cast_ns(ShutdownSingleton(tx));
                let _ = rx.await;
            }
        })?;
        let cluster = Cluster::get(context.system()).clone();
        let myself = Self {
            cluster,
            singleton_props: props,
            termination_message,
            settings,
            singleton: None,
            singleton_shutdown_notifier: None,
            singleton_keep_alive_adapter,
        };
        Ok(myself)
    }

    fn singleton_name(&self) -> &String {
        &self.settings.singleton_name
    }

    pub fn props(props: PropsBuilder<()>, termination_message: DynMessage, settings: ClusterSingletonManagerSettings) -> anyhow::Result<Props> {
        if !termination_message.cloneable() {
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
    async fn started(&mut self, _context: &mut Context) -> anyhow::Result<()> {
        Ok(())
    }

    async fn stopped(&mut self, _context: &mut Context) -> anyhow::Result<()> {
        Ok(())
    }

    async fn on_recv(&mut self, context: &mut Context, message: DynMessage) -> anyhow::Result<()> {
        Self::handle_message(self, context, message).await
    }
}
use std::any::type_name;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::OnceLock;

use async_trait::async_trait;
use tracing::{debug, warn};

use actor_cluster::cluster::Cluster;
use actor_cluster::member::Member;
use actor_cluster::unique_address::UniqueAddress;
use actor_core::{Actor, CodecMessage, DynMessage};
use actor_core::actor::context::{ActorContext1, ActorContext, ContextExt};
use actor_core::actor::props::Props;
use actor_core::actor::scheduler::ScheduleKey;
use actor_core::actor_path::TActorPath;
use actor_core::actor_ref::{ActorRef, ActorRefExt};
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;

use crate::singleton::cluster_singleton_proxy::actor_identity_wrap::ActorIdentityWrap;
use crate::singleton::cluster_singleton_proxy::cluster_event_wrap::ClusterEventWrap;
use crate::singleton::cluster_singleton_proxy::cluster_singleton_proxy_settings::ClusterSingletonProxySettings;
use crate::singleton::cluster_singleton_proxy::singleton_terminated::SingletonTerminated;
use crate::singleton::cluster_singleton_proxy::try_to_identify_singleton::TryToIdentifySingleton;

pub mod cluster_singleton_proxy_settings;
mod cluster_event_wrap;
mod try_to_identify_singleton;
mod actor_identity_wrap;
mod singleton_terminated;

static ALL_PROXY_MESSAGE: OnceLock<HashSet<&'static str>> = OnceLock::new();


#[derive(Debug)]
pub struct ClusterSingletonProxy {
    singleton_mgr_path: String,
    settings: ClusterSingletonProxySettings,
    host_singleton_members: HashMap<UniqueAddress, Member>,
    singleton: Option<ActorRef>,
    buffer: VecDeque<(DynMessage, Option<ActorRef>)>,
    identify_timer: Option<ScheduleKey>,
    cluster: Cluster,
    identify_adapter: ActorRef,
}

impl ClusterSingletonProxy {
    fn new(context: &mut ActorContext1, singleton_mgr_path: String, settings: ClusterSingletonProxySettings) -> Self {
        let identify_adapter = context.adapter(|m| DynMessage::user(ActorIdentityWrap(m)));
        let cluster = Cluster::get(context.system()).clone();
        Self {
            singleton_mgr_path,
            settings,
            host_singleton_members: Default::default(),
            singleton: None,
            buffer: Default::default(),
            identify_timer: None,
            cluster,
            identify_adapter,
        }
    }

    pub fn props(singleton_mgr_path: impl Into<String>, settings: ClusterSingletonProxySettings) -> Props {
        let singleton_mgr_path = singleton_mgr_path.into();
        Props::new_with_ctx(move |context| {
            Ok(Self::new(context, singleton_mgr_path, settings))
        })
    }

    fn cancel_timer(&mut self) {
        if let Some(timer) = self.identify_timer.take() {
            timer.cancel();
        }
    }

    fn matching_role(&self, member: &Member) -> bool {
        if let Some(role) = &self.settings.role {
            member.roles.contains(role)
        } else {
            true
        }
    }

    fn is_proxy_message(&self, message_name: &str) -> bool {
        let proxy_messages = ALL_PROXY_MESSAGE.get_or_init(|| {
            let mut messages = HashSet::new();
            messages.insert(type_name::<ClusterEventWrap>());
            messages.insert(type_name::<TryToIdentifySingleton>());
            messages.insert(type_name::<ActorIdentityWrap>());
            messages.insert(type_name::<SingletonTerminated>());
            messages
        });
        proxy_messages.contains(message_name)
    }

    fn buffer_message(&mut self, context: &mut ActorContext1, message: DynMessage) {
        let buffer_size = self.settings.buffer_size;
        let proxy_name = context.myself().path().name();
        if buffer_size <= 0 {
            debug!("{} buffer is disabled, drop current message {}", proxy_name, message.name());
        } else {
            debug!("{} buffer message {}", proxy_name, message.name());
            let sender = context.sender().cloned();
            self.buffer.push_back((message, sender));
            if self.buffer.len() > buffer_size {
                if let Some((oldest, _)) = self.buffer.pop_front() {
                    let msg_name = oldest.name();
                    let proxy_name = context.myself().path().name();
                    warn!("{} buffer full({}), drop oldest message {}", proxy_name, buffer_size, msg_name);
                }
            }
        }
    }

    fn send_buffered(&mut self) {
        if let Some(singleton) = &self.singleton {
            self.buffer.drain(..).for_each(|(msg, sender)| {
                singleton.tell(msg, sender);
            });
            debug!("send buffered messages to singleton {}", singleton);
        }
    }

    // TODO 这里是否可以优化为将当前singleton存入etcd，然后所有proxy监听此地址以此来感知singleton的变化？
    fn identify_singleton(&mut self, context: &mut ActorContext1) {
        let myself = context.myself().clone();
        let scheduler = &context.system().scheduler;
        self.cancel_timer();
        let timer = scheduler.schedule_with_fixed_delay(None, self.settings.singleton_identification_interval, move || {
            myself.cast_ns(TryToIdentifySingleton);
        });
        self.identify_timer = Some(timer);
    }

    fn singleton_paths(&self) -> Vec<String> {
        let mut paths = self.singleton_mgr_path.split("/").map(|p| p.to_string()).collect::<Vec<_>>();
        paths.push(self.settings.singleton_name.clone());
        paths
    }
}

#[async_trait]
impl Actor for ClusterSingletonProxy {
    async fn started(&mut self, context: &mut ActorContext1) -> anyhow::Result<()> {
        self.cluster.subscribe(
            context.myself().clone(),
            |event| { ClusterEventWrap(event).into_dyn() },
        )?;
        self.identify_singleton(context);
        Ok(())
    }


    async fn stopped(&mut self, context: &mut ActorContext1) -> anyhow::Result<()> {
        self.cancel_timer();
        self.cluster.unsubscribe_cluster_event(context.myself())?;
        if let Some(singleton) = &self.singleton {
            context.unwatch(singleton);
        }
        Ok(())
    }

    async fn on_recv(&mut self, context: &mut ActorContext1, message: DynMessage) -> anyhow::Result<()> {
        if self.is_proxy_message(message.name()) {
            Self::handle_message(self, context, message).await?;
        } else {
            match &self.singleton {
                None => {
                    self.buffer_message(context, message);
                }
                Some(singleton) => {
                    debug!("forward message {} to singleton {}", message.name(), singleton.path());
                    context.forward(singleton, message);
                }
            }
        }
        Ok(())
    }
}
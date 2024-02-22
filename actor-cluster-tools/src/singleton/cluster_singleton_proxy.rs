use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::OnceLock;
use std::time::Duration;

use async_trait::async_trait;
use tracing::{debug, warn};
use typed_builder::TypedBuilder;

use actor_cluster::cluster::Cluster;
use actor_cluster::cluster_event::ClusterEvent;
use actor_cluster::member::{Member, MemberStatus};
use actor_cluster::unique_address::UniqueAddress;
use actor_core::{Actor, DynMessage, Message};
use actor_core::actor::actor_path::root_actor_path::RootActorPath;
use actor_core::actor::actor_path::TActorPath;
use actor_core::actor::actor_ref::{ActorRef, ActorRefExt};
use actor_core::actor::actor_ref_factory::ActorRefFactory;
use actor_core::actor::actor_selection::ActorSelectionPath;
use actor_core::actor::context::{ActorContext, Context, ContextExt};
use actor_core::actor::props::Props;
use actor_core::actor::scheduler::ScheduleKey;
use actor_core::ext::type_name_of;
use actor_core::message::identify::{ActorIdentity, Identify};
use actor_core::message::terminated::Terminated;
use actor_derive::EmptyCodec;

static ALL_PROXY_MESSAGE: OnceLock<HashSet<&'static str>> = OnceLock::new();

#[derive(Debug, Clone, TypedBuilder)]
pub struct ClusterSingletonProxySettings {
    #[builder(default = "singleton".to_string())]
    pub singleton_name: String,
    #[builder(default = None)]
    pub role: Option<String>,
    pub singleton_identification_interval: Duration,
    pub buffer_size: usize,
}

#[derive(Debug)]
pub struct ClusterSingletonProxy {
    singleton_mgr_path: String,
    settings: ClusterSingletonProxySettings,
    host_singleton_members: HashMap<UniqueAddress, Member>,
    singleton: Option<ActorRef>,
    buffer: VecDeque<(DynMessage, Option<ActorRef>)>,
    identify_timer: Option<ScheduleKey>,
    cluster: Cluster,
    cluster_adapter: ActorRef,
    identify_adapter: ActorRef,
}

impl ClusterSingletonProxy {
    fn new(context: &mut ActorContext, singleton_mgr_path: String, settings: ClusterSingletonProxySettings) -> Self {
        let cluster_adapter = context.message_adapter(|m| DynMessage::user(ClusterEventWrap(m)));
        let identify_adapter = context.message_adapter(|m| DynMessage::user(ActorIdentityWrap(m)));
        let cluster = Cluster::get(context.system()).clone();
        Self {
            singleton_mgr_path,
            settings,
            host_singleton_members: Default::default(),
            singleton: None,
            buffer: Default::default(),
            identify_timer: None,
            cluster,
            cluster_adapter,
            identify_adapter,
        }
    }

    pub fn props(singleton_mgr_path: impl Into<String>, settings: ClusterSingletonProxySettings) -> Props {
        let singleton_mgr_path = singleton_mgr_path.into();
        Props::create(move |context| {
            Ok(Self::new(context, singleton_mgr_path.clone(), settings.clone()))
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
            messages.insert(type_name_of::<ClusterEventWrap>());
            messages.insert(type_name_of::<TryToIdentifySingleton>());
            messages.insert(type_name_of::<ActorIdentityWrap>());
            messages.insert(type_name_of::<SingletonTerminated>());
            messages
        });
        proxy_messages.contains(message_name)
    }

    fn buffer_message(&mut self, context: &mut ActorContext, message: DynMessage) {
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
    fn identify_singleton(&mut self, context: &mut ActorContext) {
        let myself = context.myself().clone();
        let scheduler = context.system().scheduler();
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
    async fn started(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        self.cluster.subscribe_cluster_event(self.cluster_adapter.clone());
        self.identify_singleton(context);
        Ok(())
    }


    async fn stopped(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        self.cancel_timer();
        self.cluster.unsubscribe_cluster_event(&self.cluster_adapter);
        if let Some(singleton) = &self.singleton {
            context.unwatch(singleton);
        }
        Ok(())
    }

    fn on_recv(&mut self, context: &mut ActorContext, message: DynMessage) -> Option<DynMessage> {
        if self.is_proxy_message(message.name()) {
            Some(message)
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
            None
        }
    }
}

#[derive(Debug, EmptyCodec)]
struct ClusterEventWrap(ClusterEvent);

#[async_trait]
impl Message for ClusterEventWrap {
    type A = ClusterSingletonProxy;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        match self.0 {
            ClusterEvent::MemberUp(m) => {
                debug!("member up {:?}", m);
                if actor.matching_role(&m) {
                    actor.host_singleton_members.insert(m.addr.clone(), m);
                    actor.identify_singleton(context);
                }
            }
            ClusterEvent::MemberPrepareForLeaving(_) => {}
            ClusterEvent::MemberLeaving(_) => {}
            ClusterEvent::MemberRemoved(m) => {
                debug!("member removed {:?}", m);
                if m.addr == actor.cluster.self_member().addr {
                    context.stop(context.myself());
                } else if actor.matching_role(&m) {
                    actor.host_singleton_members.remove(&m.addr);
                    actor.identify_singleton(context);
                }
            }
            ClusterEvent::MemberDowned(m) => {
                debug!("member downed {:?}", m);
                actor.host_singleton_members.remove(&m.addr);
                //TODO 或许只需要观察到Singleton terminated的时候才需要执行identify_singleton ?
                actor.identify_singleton(context);
            }
            ClusterEvent::CurrentClusterState { members, .. } => {
                let host_members = members
                    .into_iter()
                    .filter(|(_, m)| { m.status == MemberStatus::Up && actor.matching_role(m) })
                    .collect::<HashMap<_, _>>();
                actor.host_singleton_members.extend(host_members);
                actor.identify_singleton(context);
            }
            ClusterEvent::EtcdUnreachable => {}
        }
        Ok(())
    }
}

#[derive(Debug, EmptyCodec)]
struct TryToIdentifySingleton;

#[async_trait]
impl Message for TryToIdentifySingleton {
    type A = ClusterSingletonProxy;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        if actor.identify_timer.is_some() {
            let paths = actor.singleton_paths();
            let paths_str = paths.iter().map(|p| p.as_str()).collect::<Vec<_>>();
            for (_, member) in &actor.host_singleton_members {
                let singleton_path = RootActorPath::new(member.address().clone(), "/").descendant(paths_str.clone());
                let selection = context.actor_selection(ActorSelectionPath::FullPath(singleton_path))?;
                debug!("try to identify singleton at [{}]", selection);
                selection.tell(DynMessage::system(Identify), Some(actor.identify_adapter.clone()));
            }
        }
        Ok(())
    }
}

#[derive(Debug, EmptyCodec)]
struct ActorIdentityWrap(ActorIdentity);

#[async_trait]
impl Message for ActorIdentityWrap {
    type A = ClusterSingletonProxy;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        if let Some(singleton) = self.0.actor_ref {
            if !context.is_watching(&singleton) {
                context.watch(SingletonTerminated(singleton.clone()));
            }
            actor.singleton = Some(singleton);
            actor.cancel_timer();
            actor.send_buffered();
        }
        Ok(())
    }
}

#[derive(Debug, EmptyCodec)]
struct SingletonTerminated(ActorRef);

#[async_trait]
impl Message for SingletonTerminated {
    type A = ClusterSingletonProxy;

    async fn handle(self: Box<Self>, _context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
        if let Some(singleton) = &actor.singleton {
            if *singleton == self.0 {
                debug!("singleton {} terminated", singleton);
                actor.singleton = None;
            }
        }
        Ok(())
    }
}

impl Terminated for SingletonTerminated {
    fn actor(&self) -> &ActorRef {
        &self.0
    }
}
use std::any::Any;
use std::any::type_name;
use std::fmt::{Debug, Formatter};

use anyhow::anyhow;
use async_trait::async_trait;
use bincode::{Decode, Encode};
use tracing::{error, info};

#[cfg(feature = "derive")]
pub use actor_derive::{self, *};

use crate::actor::context::{ActorContext, Context};
use crate::actor::directive::Directive;
use crate::actor_ref::ActorRef;
use crate::delegate::downcast_box_message;
use crate::delegate::system::SystemDelegate;
use crate::delegate::user::UserDelegate;
use crate::message::message_registry::MessageRegistry;
use crate::message::MessageDecoder;

pub(crate) const CORE_CONFIG: &'static str = include_str!("../core.toml");

pub mod ext;
mod cell;
pub mod delegate;
pub mod message;
pub mod event;
pub mod routing;
pub mod actor;
pub mod config;
pub mod pattern;
pub mod actor_path;
pub mod actor_ref;
pub mod provider;
pub mod async_ref;

#[async_trait]
pub trait Actor: Send + Any {
    #[allow(unused_variables)]
    async fn started(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    async fn stopped(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    fn on_child_failure(&mut self, context: &mut ActorContext, child: &ActorRef, error: &anyhow::Error) -> Directive {
        Directive::Resume
    }

    #[allow(unused_variables)]
    async fn on_recv(&mut self, context: &mut ActorContext, message: DynMessage) -> anyhow::Result<()>;

    async fn handle_message<A: Actor>(
        actor: &mut A,
        context: &mut ActorContext,
        message: DynMessage,
    ) -> anyhow::Result<()>
        where
            Self: Sized
    {
        match message.downcast_user_delegate::<A>() {
            Ok(message) => {
                message.handle(context, actor).await?;
            }
            Err(error) => {
                error!("{:?}", error);
            }
        }
        Ok(())
    }
}

pub trait CodecMessage: Any + Send {
    fn into_any(self: Box<Self>) -> Box<dyn Any>;

    fn as_any(&self) -> &dyn Any;

    fn into_codec(self: Box<Self>) -> Box<dyn CodecMessage>;

    fn decoder() -> Option<Box<dyn MessageDecoder>> where Self: Sized;

    fn encode(self: Box<Self>, reg: &MessageRegistry) -> anyhow::Result<Vec<u8>>;

    fn clone_box(&self) -> anyhow::Result<Box<dyn CodecMessage>>;

    fn cloneable(&self) -> bool;

    fn into_dyn(self) -> DynMessage;
}

#[async_trait]
pub trait Message: CodecMessage {
    type A: Actor;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()>;
}

#[async_trait]
pub trait SystemMessage: CodecMessage {
    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut dyn Actor) -> anyhow::Result<()>;
}

pub trait OrphanMessage: CodecMessage {}

#[derive(Debug, Copy, Clone, Encode, Decode)]
pub enum MessageType {
    User,
    System,
    Orphan,
}

pub struct DynMessage {
    name: &'static str,
    ty: MessageType,
    message: Box<dyn CodecMessage>,
}

impl Debug for DynMessage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DynMessage")
            .field("name", &self.name)
            .field("ty", &self.ty)
            .finish_non_exhaustive()
    }
}

impl DynMessage {
    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn ty(&self) -> &MessageType {
        &self.ty
    }

    pub fn message(&self) -> &Box<dyn CodecMessage> {
        &self.message
    }

    pub fn into_inner(self) -> Box<dyn CodecMessage> {
        self.message
    }

    pub fn new<M>(name: &'static str, ty: MessageType, message: M) -> Self where M: CodecMessage {
        DynMessage {
            name,
            ty,
            message: Box::new(message),
        }
    }

    pub fn dyn_clone(&self) -> anyhow::Result<DynMessage> {
        let dyn_message = DynMessage {
            name: self.name,
            ty: self.ty,
            message: self.message.clone_box()?,
        };
        Ok(dyn_message)
    }

    pub fn cloneable(&self) -> bool {
        self.message.cloneable()
    }

    pub fn user<M>(message: M) -> Self where M: Message {
        let delegate = UserDelegate::new(message);
        DynMessage::new(delegate.name, MessageType::User, delegate)
    }

    pub fn system<M>(message: M) -> Self where M: SystemMessage {
        let delegate = SystemDelegate::new(message);
        DynMessage::new(delegate.name, MessageType::System, delegate)
    }

    pub fn orphan<M>(message: M) -> Self where M: OrphanMessage {
        let name = type_name::<M>();
        DynMessage::new(name, MessageType::Orphan, message)
    }

    /// 判断[`DynMessage`]的实际消息类型，大部分消息都会包装一层代理层，用于downcast到具体的类型，因为Rust不允许从一个trait object
    /// downcast到另外一个trait object，所以要包装一层具体的类型，这里直接取[`DynMessage::name`]进行比较，这里存放的是原始的消息名称
    pub fn is<M>(&self) -> bool where M: CodecMessage {
        let name = type_name::<M>();
        self.name() == name
    }

    pub fn downcast_user_delegate<A>(self) -> anyhow::Result<Box<UserDelegate<A>>> where A: Actor {
        let Self { name, ty, message } = self;
        let message = message.into_any();
        let user_delegate = if matches!(ty, MessageType::User) {
            message.downcast::<UserDelegate<A>>()
                .map_err(|_| anyhow!("message {} cannot downcast to UserDelegate<{}>", name, type_name::<A>()))
        } else {
            Err(anyhow!("message {} is not a user message", name))
        };
        user_delegate
    }

    pub fn downcast_system_delegate(self) -> anyhow::Result<Box<SystemDelegate>> {
        let Self { name, ty, message } = self;
        let message = message.into_any();
        let system_delegate = if matches!(ty, MessageType::System) {
            message.downcast::<SystemDelegate>()
                .map_err(|_| anyhow!("message {} cannot downcast to {}", name, type_name::<SystemDelegate>()))
        } else {
            Err(anyhow!("message {} is not a user message", name))
        };
        system_delegate
    }

    pub fn downcast_user_delegate_ref<A>(&self) -> Option<&UserDelegate<A>> where A: Actor {
        let Self { ty, message, .. } = self;
        let message = message.as_any();
        if matches!(ty, MessageType::User) {
            message.downcast_ref::<UserDelegate<A>>()
        } else {
            None
        }
    }

    pub fn downcast_system_delegate_ref(&self) -> Option<&SystemDelegate> {
        let Self { ty, message, .. } = self;
        let message = message.as_any();
        if matches!(ty, MessageType::System) {
            message.downcast_ref::<SystemDelegate>()
        } else {
            None
        }
    }

    pub fn downcast_user<A, M>(self) -> anyhow::Result<M> where A: Actor, M: Message {
        let message: M = self.downcast_user_delegate::<A>().map(|d| d.downcast())??;
        Ok(message)
    }

    pub fn downcast_user_ref<A, M>(&self) -> Option<&M> where A: Actor, M: Message {
        self.downcast_user_delegate_ref::<A>().map(|d| d.downcast_ref()).unwrap_or_default()
    }

    pub fn downcast_system<M>(self) -> anyhow::Result<M> where M: SystemMessage {
        let message: M = self.downcast_system_delegate().map(|d| d.downcast())??;
        Ok(message)
    }

    pub fn downcast_system_ref<M>(&self) -> Option<&M> where M: SystemMessage {
        self.downcast_system_delegate_ref().map(|d| d.downcast_ref()).unwrap_or_default()
    }

    pub fn downcast_orphan<M>(self) -> anyhow::Result<M> where M: OrphanMessage {
        let Self { name, message, .. } = self;
        downcast_box_message(name, message.into_any())
    }

    pub fn downcast_orphan_ref<M>(&self) -> Option<&M> where M: OrphanMessage {
        let Self { message, .. } = self;
        message.as_any().downcast_ref()
    }
}

#[derive(Debug)]
pub struct EmptyTestActor;

#[async_trait]
impl Actor for EmptyTestActor {
    async fn started(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        info!("{} started", context.myself());
        Ok(())
    }

    async fn stopped(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        info!("{} stopped", context.myself());
        Ok(())
    }

    async fn on_recv(&mut self, context: &mut ActorContext, message: DynMessage) -> anyhow::Result<()> {
        Self::handle_message(self, context, message).await
    }
}

#[cfg(test)]
mod actor_test {
    use std::time::Duration;

    use anyhow::anyhow;
    use async_trait::async_trait;
    use tracing::{info, Level};

    use actor_derive::{EmptyCodec, OrphanEmptyCodec};

    use crate::{Actor, DynMessage, EmptyTestActor, Message};
    use crate::actor::actor_system::ActorSystem;
    use crate::actor::context::{ActorContext, Context};
    use crate::actor::props::Props;
    use crate::actor_ref::{ActorRef, ActorRefExt};
    use crate::actor_ref::actor_ref_factory::ActorRefFactory;
    use crate::config::actor_setting::ActorSetting;
    use crate::ext::init_logger;

    #[ctor::ctor]
    fn init() {
        init_logger(Level::DEBUG)
    }

    #[tokio::test]
    async fn test_death_watch() -> anyhow::Result<()> {
        #[derive(Debug)]
        struct DeathWatchActor {
            depth: usize,
        }

        #[async_trait]
        impl Actor for DeathWatchActor {
            async fn started(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
                info!("{} started", context.myself);
                for _ in 0..3 {
                    let n = self.depth - 1;
                    if n > 0 {
                        context.spawn_anonymous(Props::new(move || Ok(DeathWatchActor { depth: n })))?;
                    }
                }
                Ok(())
            }

            async fn stopped(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
                info!("{} stopped", context.myself);
                Ok(())
            }

            async fn on_recv(&mut self, context: &mut ActorContext, message: DynMessage) -> anyhow::Result<()> {
                Self::handle_message(self, context, message).await
            }
        }

        let system = ActorSystem::new("mikai233", ActorSetting::default())?;
        let actor = system.spawn_anonymous(Props::new(|| Ok(DeathWatchActor { depth: 3 })))?;
        tokio::time::sleep(Duration::from_secs(1)).await;
        system.stop(&actor);
        tokio::time::sleep(Duration::from_secs(3)).await;
        system.terminate().await;
        Ok(())
    }

    // #[tokio::test]
    // async fn test_watch() -> anyhow::Result<()> {
    //     #[derive(Debug, EmptyCodec)]
    //     struct WatchActorTerminate {
    //         watch: ActorRef,
    //     }
    //
    //     impl WatchTerminated for WatchActorTerminate {
    //         fn watch_actor(&self) -> &ActorRef {
    //             &self.watch
    //         }
    //     }
    //
    //     #[async_trait]
    //     impl Message for WatchActorTerminate {
    //         type A = EmptyTestActor;
    //
    //         async fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
    //             info!("{} watch actor {} terminate", context.myself, self.watch);
    //             Ok(())
    //         }
    //     }
    //
    //     #[derive(Debug, EmptyCodec)]
    //     struct WatchFor {
    //         actor: ActorRef,
    //     }
    //
    //     #[async_trait]
    //     impl Message for WatchFor {
    //         type A = EmptyTestActor;
    //
    //         async fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
    //             info!("{} watch {}", context.myself, self.actor);
    //             let watch = WatchActorTerminate {
    //                 watch: self.actor,
    //             };
    //             context.watch(watch);
    //             Ok(())
    //         }
    //     }
    //
    //     #[derive(Debug, EmptyCodec)]
    //     struct UnwatchFor {
    //         actor: ActorRef,
    //     }
    //
    //     #[async_trait]
    //     impl Message for UnwatchFor {
    //         type A = EmptyTestActor;
    //
    //         async fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
    //             info!("{} unwatch {}", context.myself, self.actor);
    //             context.unwatch(&self.actor);
    //             Ok(())
    //         }
    //     }
    //
    //     fn build_config(addr: SocketAddrV4) -> ActorSetting {
    //         let mut settings = ActorSetting::builder()
    //             .provider_fn(|s| {
    //                 Remote
    //             })
    //         settings.with_provider(move |system| {
    //             let mut registration = MessageRegistration::new();
    //             registration.register::<Ping>();
    //             registration.register::<Pong>();
    //             RemoteActorRefProvider::new(system, registration, addr).map(|(r, d)| (r.into(), d))
    //         });
    //         settings
    //     }
    //
    //     let system1 = ActorSystem::create(build_config("127.0.0.1:12121".parse()?)).await?;
    //     let system2 = ActorSystem::create(build_config("127.0.0.1:12122".parse()?)).await?;
    //     let props = Props::create(|_| EmptyTestActor);
    //     let system1_actor = system1.spawn_anonymous_actor(props.clone())?;
    //     let system2_actor1 = system2.spawn_anonymous_actor(props.clone())?;
    //     let system2_actor2 = system2.spawn_anonymous_actor(props.clone())?;
    //     tokio::time::sleep(Duration::from_secs(1)).await;
    //     system1_actor.cast(WatchFor { actor: system2_actor1.clone() }, None);
    //     system1_actor.cast(WatchFor { actor: system2_actor2.clone() }, None);
    //     system1_actor.cast(UnwatchFor { actor: system2_actor2.clone() }, None);
    //     tokio::time::sleep(Duration::from_secs(1)).await;
    //     system2_actor1.stop();
    //     system2_actor2.stop();
    //     tokio::time::sleep(Duration::from_secs(3)).await;
    //     Ok(())
    // }

    #[test]
    fn derive_test() {
        #[derive(EmptyCodec)]
        struct LocalMessage;

        #[async_trait]
        impl Message for LocalMessage {
            type A = EmptyTestActor;

            async fn handle(self: Box<Self>, _context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
                Ok(())
            }
        }
    }

    #[tokio::test]
    async fn test_adapter() -> anyhow::Result<()> {
        #[derive(OrphanEmptyCodec)]
        struct TestOrphanMessage;

        #[derive(EmptyCodec)]
        struct TestMessage;

        #[async_trait]
        impl Message for TestMessage {
            type A = AdapterActor;

            async fn handle(self: Box<Self>, _context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
                info!("get transform message");
                Ok(())
            }
        }
        struct AdapterActor;

        #[async_trait]
        impl Actor for AdapterActor {
            async fn started(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
                let adapter = context.adapter::<TestOrphanMessage>(|_| {
                    DynMessage::user(TestMessage)
                });
                adapter.tell(DynMessage::orphan(TestOrphanMessage), ActorRef::no_sender());
                Ok(())
            }

            async fn on_recv(&mut self, context: &mut ActorContext, message: DynMessage) -> anyhow::Result<()> {
                Self::handle_message(self, context, message).await
            }
        }
        let system = ActorSystem::new("mikai233", ActorSetting::default())?;
        system.spawn_anonymous(Props::new(|| Ok(AdapterActor)))?;
        tokio::time::sleep(Duration::from_secs(1)).await;
        system.terminate().await;
        Ok(())
    }

    #[tokio::test]
    async fn test_cancellation() -> anyhow::Result<()> {
        #[derive(Debug)]
        struct TestActor;

        #[async_trait]
        impl Actor for TestActor {
            async fn started(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
                Ok(())
            }

            async fn stopped(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
                info!("{} stopped", context.myself());
                Ok(())
            }

            async fn on_recv(&mut self, context: &mut ActorContext, message: DynMessage) -> anyhow::Result<()> {
                Self::handle_message(self, context, message).await
            }
        }

        #[derive(Debug, EmptyCodec)]
        struct TestMessage;

        #[async_trait]
        impl Message for TestMessage {
            type A = TestActor;

            async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
                tokio::time::sleep(Duration::from_secs(2)).await;
                Err(anyhow!("should be cancelled"))
            }
        }

        let system = ActorSystem::new("mikai233", ActorSetting::default())?;
        let actor_ref = system.spawn_anonymous(Props::new(|| Ok(TestActor)))?;
        actor_ref.cast_ns(TestMessage);
        system.terminate().await;
        Ok(())
    }
}
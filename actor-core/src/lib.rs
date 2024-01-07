extern crate core;

use std::any::Any;
use std::fmt::{Debug, Formatter};

use anyhow::anyhow;
use async_trait::async_trait;
use bincode::{Decode, Encode};
use bincode::error::EncodeError;
use tracing::info;

use actor::decoder::MessageDecoder;
use actor_derive::MessageCodec;

use crate::actor::actor_ref_factory::ActorRefFactory;
use crate::actor::context::{ActorContext, Context};
use crate::actor::fault_handing::{default_strategy, SupervisorStrategy};
use crate::delegate::downcast_box_message;
use crate::delegate::system::SystemDelegate;
use crate::delegate::user::UserDelegate;
use crate::message::message_registration::MessageRegistration;

pub mod ext;
mod cell;
pub mod delegate;
pub mod message;
mod event;
pub mod routing;
mod indirect_actor_producer;
pub mod actor;

#[async_trait]
pub trait Actor: Send + Any {
    #[allow(unused_variables)]
    async fn pre_start(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    async fn post_stop(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        Ok(())
    }

    async fn pre_restart(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        let children = context.children();
        for child in children {
            context.unwatch(&child);
            context.stop(&child);
        }
        self.post_stop(context).await
    }

    fn supervisor_strategy(&self) -> Box<dyn SupervisorStrategy> {
        default_strategy()
    }

    #[allow(unused_variables)]
    fn handle_message(&mut self, context: &mut ActorContext, message: DynMessage) -> Option<DynMessage> {
        Some(message)
    }
}

pub trait CodecMessage: Any + Send {
    fn into_any(self: Box<Self>) -> Box<dyn Any>;

    fn as_any(&self) -> &dyn Any;

    fn decoder() -> Option<Box<dyn MessageDecoder>> where Self: Sized;

    fn encode(&self, reg: &MessageRegistration) -> Result<Vec<u8>, EncodeError>;

    fn dyn_clone(&self) -> Option<DynMessage>;
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
    pub name: &'static str,
    pub message_type: MessageType,
    pub boxed: Box<dyn CodecMessage>,
}

impl Debug for DynMessage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DynMessage")
            .field("name", &self.name)
            .field("message_type", &self.message_type)
            .field("boxed", &"..")
            .finish()
    }
}

impl DynMessage {
    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn new<M>(name: &'static str, message_type: MessageType, message: M) -> Self where M: CodecMessage {
        DynMessage {
            name,
            message_type,
            boxed: Box::new(message),
        }
    }

    pub fn dyn_clone(&self) -> Option<DynMessage> {
        self.boxed.dyn_clone()
    }

    pub fn user<M>(message: M) -> Self where M: Message {
        let delegate = UserDelegate::new(message);
        DynMessage::new(delegate.name, MessageType::User, delegate)
    }

    pub(crate) fn system<M>(message: M) -> Self where M: SystemMessage {
        let delegate = SystemDelegate::new(message);
        DynMessage::new(delegate.name, MessageType::System, delegate)
    }

    pub fn orphan<M>(message: M) -> Self where M: OrphanMessage {
        let name = std::any::type_name::<M>();
        DynMessage::new(name, MessageType::Orphan, message)
    }

    /// 判断[`DynMessage`]的实际消息类型，大部分消息都会包装一层代理层，用于downcast到具体的类型，因为Rust不允许从一个trait object
    /// downcast到另外一个trait object，所以要包装一层具体的类型，这里直接取[`DynMessage::name`]进行比较，这里存放的是原始的消息名称
    pub fn is<M>(&self) -> bool where M: CodecMessage {
        let name = std::any::type_name::<M>();
        self.name() == name
    }

    pub fn downcast_user_delegate<A>(self) -> anyhow::Result<Box<UserDelegate<A>>> where A: Actor {
        let Self { name, message_type, boxed } = self;
        let message = boxed.into_any();
        let user_delegate = if matches!(message_type, MessageType::User) {
            message.downcast::<UserDelegate<A>>()
                .map_err(|_| anyhow!("message {} cannot downcast to UserDelegate<{}>", name, std::any::type_name::<A>()))
        } else {
            Err(anyhow!("message {} is not a user message", name))
        };
        user_delegate
    }

    pub fn downcast_system_delegate(self) -> anyhow::Result<Box<SystemDelegate>> {
        let Self { name, message_type, boxed } = self;
        let message = boxed.into_any();
        let system_delegate = if matches!(message_type, MessageType::System) {
            message.downcast::<SystemDelegate>()
                .map_err(|_| anyhow!("message {} cannot downcast to SystemDelegate", name))
        } else {
            Err(anyhow!("message {} is not a user message", name))
        };
        system_delegate
    }

    pub fn downcast_user_delegate_ref<A>(&self) -> Option<&UserDelegate<A>> where A: Actor {
        let Self { message_type, boxed, .. } = self;
        let message = boxed.as_any();
        if matches!(message_type, MessageType::User) {
            message.downcast_ref::<UserDelegate<A>>()
        } else {
            None
        }
    }

    pub fn downcast_system_delegate_ref(&self) -> Option<&SystemDelegate> {
        let Self { message_type, boxed, .. } = self;
        let message = boxed.as_any();
        if matches!(message_type, MessageType::System) {
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
        let Self { name, boxed, .. } = self;
        downcast_box_message(name, boxed.into_any())
    }

    pub fn downcast_orphan_ref<M>(&self) -> Option<&M> where M: OrphanMessage {
        let Self { boxed, .. } = self;
        boxed.as_any().downcast_ref()
    }
}

#[derive(Debug)]
pub struct EmptyTestActor;

#[async_trait]
impl Actor for EmptyTestActor {
    async fn pre_start(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        info!("{} pre start", context.myself());
        Ok(())
    }

    async fn post_stop(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        info!("{} post stop", context.myself());
        Ok(())
    }
}

#[derive(Debug, Encode, Decode, MessageCodec)]
pub struct EmptyTestMessage;

#[async_trait]
impl Message for EmptyTestMessage {
    type A = EmptyTestActor;

    async fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
        info!("{} handle {:?}", context.myself(), self);
        Ok(())
    }
}

// #[cfg(test)]
// mod actor_test {
//     use std::net::SocketAddrV4;
//     use std::time::Duration;
//
//     use anyhow::Ok;
//     use async_trait::async_trait;
//     use tracing::{info, Level};
//
//     use actor_derive::{EmptyCodec, MessageCodec, UntypedMessageCodec};
//
//     use crate::{Actor, DynMessage, EmptyTestActor, Message};
//     use crate::actor::actor_ref::ActorRef;
//     use crate::actor::actor_ref_factory::ActorRefFactory;
//     use crate::actor::actor_system::ActorSystem;
//     use crate::actor::config::actor_system_config::ActorSystemConfig;
//     use crate::actor::context::{ActorContext, Context};
//     use crate::actor::props::Props;
//     use crate::ext::init_logger;
//     use crate::message::terminated::WatchTerminated;
//
//     #[ctor::ctor]
//     fn init() {
//         init_logger(Level::DEBUG)
//     }
//
//     #[tokio::test]
//     async fn test_death_watch() -> anyhow::Result<()> {
//         #[derive(Debug)]
//         struct DeathWatchActor {
//             depth: usize,
//         }
//
//         #[async_trait]
//         impl Actor for DeathWatchActor {
//             async fn pre_start(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
//                 info!("actor {} pre start", context.myself);
//                 for _ in 0..3 {
//                     let n = self.depth - 1;
//                     if n > 0 {
//                         context.spawn_anonymous_actor(Props::create(move |_| DeathWatchActor { depth: n }))?;
//                     }
//                 }
//                 Ok(())
//             }
//
//             async fn post_stop(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
//                 info!("actor {} post stop",context.myself);
//                 Ok(())
//             }
//         }
//
//         let system = ActorSystem::create("mikai233", ActorSystemConfig::default()).await?;
//         let actor = system.spawn_anonymous_actor(Props::create(|_| DeathWatchActor { depth: 3 }))?;
//         tokio::time::sleep(Duration::from_secs(1)).await;
//         system.stop(&actor);
//         tokio::time::sleep(Duration::from_secs(3)).await;
//         Ok(())
//     }
//
//     #[tokio::test]
//     async fn test_watch() -> anyhow::Result<()> {
//         #[derive(Debug, EmptyCodec)]
//         struct WatchActorTerminate {
//             watch: ActorRef,
//         }
//
//         impl WatchTerminated for WatchActorTerminate {
//             fn watch_actor(&self) -> &ActorRef {
//                 &self.watch
//             }
//         }
//
//         impl Message for WatchActorTerminate {
//             type A = EmptyTestActor;
//
//             fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
//                 info!("{} watch actor {} terminate", context.myself, self.watch);
//                 Ok(())
//             }
//         }
//
//         #[derive(Debug, EmptyCodec)]
//         struct WatchFor {
//             actor: ActorRef,
//         }
//
//         impl Message for WatchFor {
//             type A = EmptyTestActor;
//
//             fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
//                 info!("{} watch {}", context.myself, self.actor);
//                 let watch = WatchActorTerminate {
//                     watch: self.actor,
//                 };
//                 context.watch(watch);
//                 Ok(())
//             }
//         }
//
//         #[derive(Debug, EmptyCodec)]
//         struct UnwatchFor {
//             actor: ActorRef,
//         }
//
//         impl Message for UnwatchFor {
//             type A = EmptyTestActor;
//
//             fn handle(self: Box<Self>, context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
//                 info!("{} unwatch {}", context.myself, self.actor);
//                 context.unwatch(&self.actor);
//                 Ok(())
//             }
//         }
//
//         fn build_config(addr: SocketAddrV4) -> ActorSystemConfig {
//             let mut config = ActorSystemConfig::default();
//             config.with_provider(move |system| {
//                 let mut registration = MessageRegistration::new();
//                 registration.register::<Ping>();
//                 registration.register::<Pong>();
//                 RemoteActorRefProvider::new(system, registration, addr).map(|(r, d)| (r.into(), d))
//             });
//             config
//         }
//
//         let system1 = ActorSystem::create(build_config("127.0.0.1:12121".parse()?)).await?;
//         let system2 = ActorSystem::create(build_config("127.0.0.1:12122".parse()?)).await?;
//         let props = Props::create(|_| EmptyTestActor);
//         let system1_actor = system1.spawn_anonymous_actor(props.clone())?;
//         let system2_actor1 = system2.spawn_anonymous_actor(props.clone())?;
//         let system2_actor2 = system2.spawn_anonymous_actor(props.clone())?;
//         tokio::time::sleep(Duration::from_secs(1)).await;
//         system1_actor.cast(WatchFor { actor: system2_actor1.clone() }, None);
//         system1_actor.cast(WatchFor { actor: system2_actor2.clone() }, None);
//         system1_actor.cast(UnwatchFor { actor: system2_actor2.clone() }, None);
//         tokio::time::sleep(Duration::from_secs(1)).await;
//         system2_actor1.stop();
//         system2_actor2.stop();
//         tokio::time::sleep(Duration::from_secs(3)).await;
//         Ok(())
//     }
//
//     #[test]
//     fn derive_test() {
//         #[derive(EmptyCodec)]
//         struct LocalMessage;
//
//         impl Message for LocalMessage {
//             type A = EmptyTestActor;
//
//             fn handle(self: Box<Self>, _context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
//                 Ok(())
//             }
//         }
//     }
//
//     #[tokio::test]
//     async fn test_adapter() -> anyhow::Result<()> {
//         #[derive(Serialize, Deserialize, UntypedMessageCodec)]
//         struct TestUntyped;
//
//         #[derive(Serialize, Deserialize, MessageCodec)]
//         struct TestMessage;
//
//         impl Message for TestMessage {
//             type A = AdapterActor;
//
//             fn handle(self: Box<Self>, _context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
//                 info!("get transform message");
//                 Ok(())
//             }
//         }
//         struct AdapterActor;
//
//         #[async_trait]
//         impl Actor for AdapterActor {
//             async fn pre_start(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
//                 let adapter = context.message_adapter::<TestUntyped>(|_| {
//                     Ok(DynMessage::user(TestMessage))
//                 });
//                 adapter.tell(DynMessage::untyped(TestUntyped), ActorRef::no_sender());
//                 Ok(())
//             }
//         }
//         fn build_config() -> ActorSystemConfig {
//             let mut config = ActorSystemConfig::default();
//             config.registration.register::<TestMessage>();
//             config.registration.register::<TestUntyped>();
//             config
//         }
//         let system = ActorSystem::create(build_config()).await?;
//         system.spawn_anonymous_actor(Props::create(|_| AdapterActor))?;
//         tokio::time::sleep(Duration::from_secs(3)).await;
//         Ok(())
//     }
// }

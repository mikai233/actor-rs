use std::any::Any;
use std::fmt::{Debug, Formatter};

use anyhow::anyhow;
use async_trait::async_trait;
use dyn_clone::DynClone;
use serde::{Deserialize, Serialize};
use tracing::info;

use actor_derive::MessageCodec;

use crate::context::{ActorContext, Context};
use crate::decoder::MessageDecoder;
use crate::delegate::MessageDelegate;
use crate::delegate::system::SystemDelegate;
use crate::delegate::user::{AsyncUserDelegate, UserDelegate};

pub mod actor_ref;
pub mod system;
pub(crate) mod state;
pub mod props;
pub(crate) mod actor_path;
pub(crate) mod address;
pub mod provider;
pub mod ext;
mod net;
mod cell;
mod cluster;
pub mod decoder;
pub mod delegate;
pub mod message;
pub mod context;
mod event;
pub mod routing;
mod indirect_actor_producer;

#[async_trait]
pub trait Actor: Send + Sync + Sized + 'static {
    type S: State;
    type A: Arg;
    async fn pre_start(context: &mut ActorContext, arg: Self::A) -> anyhow::Result<Self::S>;

    #[allow(unused_variables)]
    async fn post_stop(context: &mut ActorContext, state: &mut Self::S) -> anyhow::Result<()> {
        Ok(())
    }
}

pub trait CodecMessage: Any + Send {
    fn into_any(self: Box<Self>) -> Box<dyn Any>;
    fn decoder() -> Option<Box<dyn MessageDecoder>> where Self: Sized;
    fn encode(&self) -> Option<anyhow::Result<Vec<u8>>>;
    fn dyn_clone(&self) -> Option<DynMessage>;
}

pub trait Message: CodecMessage {
    type T: Actor;

    fn handle(self: Box<Self>, context: &mut ActorContext, state: &mut <Self::T as Actor>::S) -> anyhow::Result<()>;
}

#[async_trait]
pub trait AsyncMessage: CodecMessage {
    type T: Actor;

    async fn handle(self: Box<Self>, context: &mut ActorContext, state: &mut <Self::T as Actor>::S) -> anyhow::Result<()>;
}

#[async_trait]
pub(crate) trait SystemMessage: CodecMessage {
    async fn handle(self: Box<Self>, context: &mut ActorContext) -> anyhow::Result<()>;
}

pub trait UntypedMessage: CodecMessage {}

#[derive(Debug, Copy, Clone)]
pub enum MessageType {
    User,
    AsyncUser,
    System,
    Untyped,
}

pub struct DynMessage {
    pub(crate) name: &'static str,
    pub(crate) message_type: MessageType,
    pub(crate) boxed: Box<dyn CodecMessage>,
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

    pub fn clone(&self) -> Option<DynMessage> {
        self.boxed.dyn_clone()
    }
}

impl DynMessage {
    pub fn user<M>(message: M) -> Self where M: Message {
        let delegate = UserDelegate::new(message);
        DynMessage::new(delegate.name, MessageType::User, delegate)
    }

    pub fn async_user<M>(message: M) -> Self where M: AsyncMessage {
        let delegate = AsyncUserDelegate::new(message);
        DynMessage::new(delegate.name, MessageType::AsyncUser, delegate)
    }

    pub fn system<M>(message: M) -> Self where M: SystemMessage {
        let delegate = SystemDelegate::new(message);
        DynMessage::new(delegate.name, MessageType::System, delegate)
    }

    pub fn untyped<M>(message: M) -> Self where M: UntypedMessage {
        let name = std::any::type_name::<M>();
        DynMessage::new(name, MessageType::Untyped, message)
    }

    pub(crate) fn downcast_into_delegate<T>(self) -> anyhow::Result<MessageDelegate<T>> where T: Actor {
        let name = self.name();
        let message = self.boxed.into_any();
        let delegate = match self.message_type {
            MessageType::User => {
                message.downcast::<UserDelegate<T>>().map(|m| MessageDelegate::User(m))
            }
            MessageType::AsyncUser => {
                message.downcast::<AsyncUserDelegate<T>>().map(|m| MessageDelegate::AsyncUser(m))
            }
            MessageType::System => {
                message.downcast::<SystemDelegate>().map(|m| MessageDelegate::System(m))
            }
            MessageType::Untyped => {
                panic!("unexpected Untyped message {}", name);
            }
        };
        match delegate {
            Ok(delegate) => {
                Ok(delegate)
            }
            Err(_) => {
                Err(anyhow!("unexpected message {} to actor {}", name, std::any::type_name::<T>()))
            }
        }
    }
}


pub trait State: Any + Send {}

impl<T> State for T where T: Any + Send {}

pub trait Arg: Any + Send + DynClone {}

impl<T> Arg for T where T: Any + Send + DynClone {}

dyn_clone::clone_trait_object!(Arg);

#[derive(Debug)]
pub(crate) struct EmptyTestActor;

#[async_trait]
impl Actor for EmptyTestActor {
    type S = ();
    type A = ();

    async fn pre_start(context: &mut ActorContext, _arg: Self::A) -> anyhow::Result<Self::S> {
        info!("{} pre start", context.myself());
        Ok(())
    }

    async fn post_stop(context: &mut ActorContext, _state: &mut Self::S) -> anyhow::Result<()> {
        info!("{} post stop", context.myself());
        Ok(())
    }
}


#[derive(Debug, Serialize, Deserialize, MessageCodec)]
#[actor(EmptyTestActor)]
pub(crate) struct EmptyTestMessage;

impl Message for EmptyTestMessage {
    type T = EmptyTestActor;

    fn handle(self: Box<Self>, context: &mut ActorContext, _state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
        info!("{} handle {:?}", context.myself(), self);
        Ok(())
    }
}

#[cfg(test)]
mod actor_test {
    use std::net::SocketAddrV4;
    use std::time::{Duration, SystemTime};

    use anyhow::Ok;
    use async_trait::async_trait;
    use serde::{Deserialize, Serialize};
    use tracing::{info, Level};

    use actor_derive::{EmptyCodec, MessageCodec, UntypedMessageCodec};

    use crate::{Actor, DynMessage, EmptyTestActor, Message};
    use crate::actor_ref::{ActorRef, ActorRefExt, TActorRef};
    use crate::actor_ref::deferred_ref::Patterns;
    use crate::context::{ActorContext, Context};
    use crate::ext::init_logger;
    use crate::message::terminated::WatchTerminated;
    use crate::props::{noarg_props, props};
    use crate::provider::{ActorRefFactory, TActorRefProvider};
    use crate::system::ActorSystem;
    use crate::system::config::Config;

    #[ctor::ctor]
    fn init() {
        init_logger(Level::DEBUG)
    }

    #[tokio::test]
    async fn test_death_watch() -> anyhow::Result<()> {
        #[derive(Debug)]
        struct DeathWatchActor;

        #[async_trait]
        impl Actor for DeathWatchActor {
            type S = ();
            type A = usize;

            async fn pre_start(context: &mut ActorContext, arg: Self::A) -> anyhow::Result<Self::S> {
                info!("actor {} pre start", context.myself);
                for _ in 0..3 {
                    let n = arg - 1;
                    if n > 0 {
                        context.actor_of(props::<DeathWatchActor>(arg - 1), None)?;
                    }
                }
                Ok(())
            }

            async fn post_stop(context: &mut ActorContext, _state: &mut Self::S) -> anyhow::Result<()> {
                info!("actor {} post stop",context.myself);
                Ok(())
            }
        }

        let system = ActorSystem::create(Config::default()).await?;
        let actor = system.actor_of(props::<DeathWatchActor>(3), None)?;
        tokio::time::sleep(Duration::from_secs(1)).await;
        system.stop(&actor);
        tokio::time::sleep(Duration::from_secs(3)).await;
        Ok(())
    }

    #[tokio::test]
    async fn test_watch() -> anyhow::Result<()> {
        #[derive(Debug, EmptyCodec)]
        struct WatchActorTerminate {
            watch: ActorRef,
        }

        impl WatchTerminated for WatchActorTerminate {
            fn watch_actor(&self) -> &ActorRef {
                &self.watch
            }
        }

        impl Message for WatchActorTerminate {
            type T = EmptyTestActor;

            fn handle(self: Box<Self>, context: &mut ActorContext, _state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
                info!("{} watch actor {} terminate", context.myself, self.watch);
                Ok(())
            }
        }

        #[derive(Debug, EmptyCodec)]
        struct WatchFor {
            actor: ActorRef,
        }

        impl Message for WatchFor {
            type T = EmptyTestActor;

            fn handle(self: Box<Self>, context: &mut ActorContext, _state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
                info!("{} watch {}", context.myself, self.actor);
                let watch = WatchActorTerminate {
                    watch: self.actor,
                };
                context.watch(watch);
                Ok(())
            }
        }

        #[derive(Debug, EmptyCodec)]
        struct UnwatchFor {
            actor: ActorRef,
        }

        impl Message for UnwatchFor {
            type T = EmptyTestActor;

            fn handle(self: Box<Self>, context: &mut ActorContext, _state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
                info!("{} unwatch {}", context.myself, self.actor);
                context.unwatch(&self.actor);
                Ok(())
            }
        }

        fn build_config(addr: SocketAddrV4) -> Config {
            let mut config = Config::default();
            config.addr = addr;
            config
        }

        let system1 = ActorSystem::create(build_config("127.0.0.1:12121".parse()?)).await?;
        let system2 = ActorSystem::create(build_config("127.0.0.1:12122".parse()?)).await?;
        let system1_actor = system1.actor_of(noarg_props::<EmptyTestActor>(), None)?;
        let system2_actor1 = system2.actor_of(noarg_props::<EmptyTestActor>(), None)?;
        let system2_actor2 = system2.actor_of(noarg_props::<EmptyTestActor>(), None)?;
        tokio::time::sleep(Duration::from_secs(1)).await;
        system1_actor.cast(WatchFor { actor: system2_actor1.clone() }, None);
        system1_actor.cast(WatchFor { actor: system2_actor2.clone() }, None);
        system1_actor.cast(UnwatchFor { actor: system2_actor2.clone() }, None);
        tokio::time::sleep(Duration::from_secs(1)).await;
        system2_actor1.stop();
        system2_actor2.stop();
        tokio::time::sleep(Duration::from_secs(3)).await;
        Ok(())
    }

    #[test]
    fn derive_test() {
        #[derive(EmptyCodec)]
        struct LocalMessage;

        impl Message for LocalMessage {
            type T = EmptyTestActor;

            fn handle(self: Box<Self>, _context: &mut ActorContext, _state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
                Ok(())
            }
        }
    }

    #[tokio::test]
    async fn test_ask() -> anyhow::Result<()> {
        #[derive(Serialize, Deserialize, MessageCodec)]
        #[actor(EmptyTestActor)]
        struct MessageToAsk;

        impl Message for MessageToAsk {
            type T = EmptyTestActor;

            fn handle(self: Box<Self>, context: &mut ActorContext, _state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
                context.sender().unwrap().resp(MessageToAns {
                    content: "hello world".to_string(),
                });
                Ok(())
            }
        }

        #[derive(Serialize, Deserialize, UntypedMessageCodec)]
        struct MessageToAns {
            content: String,
        }

        fn build_config(addr: SocketAddrV4) -> Config {
            let mut config = Config::default();
            config.addr = addr;
            config.registration.register::<MessageToAsk>();
            config.registration.register::<MessageToAns>();
            config
        }

        let system1 = ActorSystem::create(build_config("127.0.0.1:12121".parse()?)).await?;
        let system2 = ActorSystem::create(build_config("127.0.0.1:12123".parse()?)).await?;
        let actor_a = system1.actor_of(noarg_props::<EmptyTestActor>(), None)?;
        let actor_a = system2.provider().resolve_actor_ref_of_path(actor_a.path());
        let start = SystemTime::now();
        for _ in 0..10000 {
            let _: MessageToAns = Patterns::ask(&actor_a, MessageToAsk, Duration::from_secs(3)).await.unwrap();
        }
        let end = SystemTime::now();
        let cost = end.duration_since(start)?;
        info!("cost {:?}", cost);
        Ok(())
    }

    #[tokio::test]
    async fn test_adapter() -> anyhow::Result<()> {
        #[derive(Serialize, Deserialize, UntypedMessageCodec)]
        struct TestUntyped;

        #[derive(Serialize, Deserialize, MessageCodec)]
        #[actor(AdapterActor)]
        struct TestMessage;

        impl Message for TestMessage {
            type T = AdapterActor;

            fn handle(self: Box<Self>, _context: &mut ActorContext, _state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
                info!("get transform message");
                Ok(())
            }
        }
        struct AdapterActor;

        #[async_trait]
        impl Actor for AdapterActor {
            type S = ();
            type A = ();

            async fn pre_start(context: &mut ActorContext, _arg: Self::A) -> anyhow::Result<Self::S> {
                let adapter = context.message_adapter::<TestUntyped>(|_| {
                    Ok(DynMessage::user(TestMessage))
                });
                adapter.tell(DynMessage::untyped(TestUntyped), ActorRef::no_sender());
                Ok(())
            }
        }
        fn build_config() -> Config {
            let mut config = Config::default();
            config.registration.register::<TestMessage>();
            config.registration.register::<TestUntyped>();
            config
        }
        let system = ActorSystem::create(build_config()).await?;
        system.actor_of(noarg_props::<AdapterActor>(), None)?;
        tokio::time::sleep(Duration::from_secs(3)).await;
        Ok(())
    }
}
#![feature(exclusive_range_pattern)]

use std::any::Any;
use std::fmt::{Debug, Formatter};

use anyhow::anyhow;
use async_trait::async_trait;
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

pub trait Actor: Send + Sized + 'static {
    type S: State;
    type A: Arg;
    fn pre_start(&self, context: &mut ActorContext, arg: Self::A) -> anyhow::Result<Self::S>;

    #[allow(unused_variables)]
    fn post_stop(&self, context: &mut ActorContext, state: &mut Self::S) -> anyhow::Result<()> {
        Ok(())
    }
}

pub trait CodecMessage: Any + Send {
    fn into_any(self: Box<Self>) -> Box<dyn Any>;
    fn decoder() -> Option<Box<dyn MessageDecoder>> where Self: Sized;
    fn encode(&self) -> Option<anyhow::Result<Vec<u8>>>;
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

pub trait DeferredMessage: CodecMessage {}

pub trait UntypedMessage: CodecMessage {}

#[derive(Debug)]
pub enum DynamicMessage {
    User(BoxedMessage),
    AsyncUser(BoxedMessage),
    System(BoxedMessage),
    Deferred(BoxedMessage),
    Untyped(BoxedMessage),
}

pub struct BoxedMessage {
    pub(crate) name: &'static str,
    pub(crate) inner: Box<dyn CodecMessage>,
}

impl Debug for BoxedMessage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BoxedMessage")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

impl BoxedMessage {
    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn new<M>(name: &'static str, message: M) -> Self where M: CodecMessage {
        BoxedMessage {
            name,
            inner: Box::new(message),
        }
    }
}

impl DynamicMessage {
    pub fn user<M>(message: M) -> Self where M: Message {
        let delegate = UserDelegate::new(message);
        DynamicMessage::User(BoxedMessage::new(delegate.name, delegate))
    }

    pub fn async_user<M>(message: M) -> Self where M: AsyncMessage {
        let delegate = AsyncUserDelegate::new(message);
        DynamicMessage::AsyncUser(BoxedMessage::new(delegate.name, delegate))
    }

    pub fn system<M>(message: M) -> Self where M: SystemMessage {
        let delegate = SystemDelegate::new(message);
        DynamicMessage::System(BoxedMessage::new(delegate.name, delegate))
    }

    pub fn deferred<M>(message: M) -> Self where M: DeferredMessage {
        let name = std::any::type_name::<M>();
        DynamicMessage::Deferred(BoxedMessage::new(name, message))
    }

    pub(crate) fn untyped<M>(message: M) -> Self where M: UntypedMessage {
        let name = std::any::type_name::<M>();
        DynamicMessage::Untyped(BoxedMessage::new(name, message))
    }

    pub(crate) fn name(&self) -> &'static str {
        match self {
            DynamicMessage::User(m) => { m.name() }
            DynamicMessage::AsyncUser(m) => { m.name() }
            DynamicMessage::System(m) => { m.name() }
            DynamicMessage::Deferred(m) => { m.name() }
            DynamicMessage::Untyped(m) => { m.name() }
        }
    }

    pub(crate) fn downcast_into_delegate<T>(self) -> anyhow::Result<MessageDelegate<T>> where T: Actor {
        let name = self.name();
        let delegate = match self {
            DynamicMessage::User(m) => {
                m.inner.into_any().downcast::<UserDelegate<T>>().map(|m| MessageDelegate::User(m))
            }
            DynamicMessage::AsyncUser(m) => {
                m.inner.into_any().downcast::<AsyncUserDelegate<T>>().map(|m| MessageDelegate::AsyncUser(m))
            }
            DynamicMessage::System(m) => {
                m.inner.into_any().downcast::<SystemDelegate>().map(|m| MessageDelegate::System(m))
            }
            DynamicMessage::Deferred(m) => {
                panic!("unexpected Deferred message {}", m.name());
            }
            DynamicMessage::Untyped(m) => {
                panic!("unexpected Untyped message {}", m.name());
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

pub trait Arg: Any + Send {}

impl<T> Arg for T where T: Any + Send {}

#[derive(Debug)]
pub(crate) struct EmptyTestActor;

impl Actor for EmptyTestActor {
    type S = ();
    type A = ();

    fn pre_start(&self, context: &mut ActorContext, _arg: Self::A) -> anyhow::Result<Self::S> {
        info!("{} pre start", context.myself());
        Ok(())
    }

    fn post_stop(&self, context: &mut ActorContext, _state: &mut Self::S) -> anyhow::Result<()> {
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
    use std::time::{Duration, SystemTime};

    use anyhow::Ok;
    use serde::{Deserialize, Serialize};
    use tracing::{info, Level};

    use actor_derive::{DeferredMessageCodec, EmptyCodec, MessageCodec, UntypedMessageCodec};

    use crate::{Actor, DynamicMessage, EmptyTestActor, Message};
    use crate::actor_ref::{ActorRef, ActorRefExt, SerializedActorRef, TActorRef};
    use crate::actor_ref::deferred_ref::Patterns;
    use crate::context::{ActorContext, Context};
    use crate::ext::init_logger;
    use crate::message::MessageRegistration;
    use crate::message::terminated::WatchTerminated;
    use crate::props::Props;
    use crate::provider::{ActorRefFactory, TActorRefProvider};
    use crate::system::ActorSystem;

    #[ctor::ctor]
    fn init() {
        init_logger(Level::DEBUG)
    }

    #[tokio::test]
    async fn test_death_watch() -> anyhow::Result<()> {
        #[derive(Debug)]
        struct DeathWatchActor;

        impl Actor for DeathWatchActor {
            type S = ();
            type A = usize;

            fn pre_start(&self, context: &mut ActorContext, arg: Self::A) -> anyhow::Result<Self::S> {
                info!("actor {} pre start", context.myself);
                for _ in 0..3 {
                    let n = arg - 1;
                    if n > 0 {
                        context.actor_of(DeathWatchActor, arg - 1, Props::default(), None)?;
                    }
                }
                Ok(())
            }

            fn post_stop(&self, context: &mut ActorContext, _state: &mut Self::S) -> anyhow::Result<()> {
                info!("actor {} post stop",context.myself);
                Ok(())
            }
        }

        let system = ActorSystem::new("game".to_string(), "127.0.0.1:12121".parse()?, MessageRegistration::new())?;
        let actor = system.actor_of(DeathWatchActor, 3, Props::default(), None)?;
        tokio::time::sleep(Duration::from_secs(1)).await;
        system.stop(&actor);
        tokio::time::sleep(Duration::from_secs(3)).await;
        Ok(())
    }

    #[tokio::test]
    async fn test_watch() -> anyhow::Result<()> {
        #[derive(Debug, Serialize, Deserialize, MessageCodec)]
        #[actor(EmptyTestActor)]
        struct WatchActorTerminate {
            watch: SerializedActorRef,
        }

        impl WatchTerminated for WatchActorTerminate {
            fn watch_actor(&self, system: &ActorSystem) -> ActorRef {
                system.provider().resolve_actor_ref(&self.watch.path)
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
                    watch: self.actor.into(),
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

        let system1 = ActorSystem::new("game".to_string(), "127.0.0.1:12121".parse().unwrap(), MessageRegistration::new())?;
        let system2 = ActorSystem::new("game".to_string(), "127.0.0.1:12122".parse().unwrap(), MessageRegistration::new())?;
        let system1_actor = system1.actor_of(EmptyTestActor, (), Props::default(), None)?;
        let system2_actor1 = system2.actor_of(EmptyTestActor, (), Props::default(), None)?;
        let system2_actor2 = system2.actor_of(EmptyTestActor, (), Props::default(), None)?;
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

        #[derive(Serialize, Deserialize, DeferredMessageCodec)]
        struct MessageToAns {
            content: String,
        }

        fn reg() -> MessageRegistration {
            let mut reg = MessageRegistration::new();
            reg.register::<MessageToAsk>();
            reg.register::<MessageToAns>();
            reg
        }

        let system1 = ActorSystem::new("game".to_string(), "127.0.0.1:12121".parse().unwrap(), reg())?;
        let system2 = ActorSystem::new("game".to_string(), "127.0.0.1:12122".parse().unwrap(), reg())?;
        let actor_a = system1.actor_of(EmptyTestActor, (), Props::default(), None)?;
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

        impl Actor for AdapterActor {
            type S = ();
            type A = ();

            fn pre_start(&self, context: &mut ActorContext, _arg: Self::A) -> anyhow::Result<Self::S> {
                let adapter = context.message_adapter::<TestUntyped>(|_| {
                    Ok(DynamicMessage::user(TestMessage))
                });
                adapter.tell(DynamicMessage::untyped(TestUntyped), ActorRef::no_sender());
                Ok(())
            }
        }
        fn reg() -> MessageRegistration {
            let mut reg = MessageRegistration::new();
            reg.register::<TestMessage>();
            reg.register::<TestUntyped>();
            reg
        }
        let system = ActorSystem::new("game".to_string(), "127.0.0.1:12121".parse().unwrap(), reg())?;
        system.actor_of(AdapterActor, (), Props::default(), None)?;
        tokio::time::sleep(Duration::from_secs(3)).await;
        Ok(())
    }
}
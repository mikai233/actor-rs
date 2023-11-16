use std::any::Any;
use std::fmt::{Debug, Formatter};

use anyhow::anyhow;
use async_trait::async_trait;
use serde::de::DeserializeOwned;

use crate::actor::context::ActorContext;
use crate::decoder::MessageDecoder;
use crate::delegate::MessageDelegate;
use crate::delegate::system::SystemDelegate;
use crate::delegate::user::{AsyncUserDelegate, UserDelegate};

pub mod context;

pub trait Actor: Send + Sized + 'static {
    type S: State;
    type A: Arg;
    fn pre_start(&self, context: &mut ActorContext, arg: Self::A) -> anyhow::Result<Self::S>;

    #[allow(unused_variables)]
    fn post_stop(&self, context: &mut ActorContext, state: &mut Self::S) -> anyhow::Result<()> {
        Ok(())
    }
}

pub trait CodecMessage: Any + Send + 'static {
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
pub trait SystemMessage: CodecMessage {
    async fn handle(self: Box<Self>, context: &mut ActorContext) -> anyhow::Result<()>;
}

#[derive(Debug)]
pub enum DynamicMessage {
    User(BoxedMessage),
    AsyncUser(BoxedMessage),
    System(BoxedMessage),
}

pub(crate) struct BoxedMessage {
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

    pub(crate) fn name(&self) -> &'static str {
        match self {
            DynamicMessage::User(m) => { m.name() }
            DynamicMessage::AsyncUser(m) => { m.name() }
            DynamicMessage::System(m) => { m.name() }
        }
    }

    pub(crate) fn downcast<T>(self) -> anyhow::Result<MessageDelegate<T>> where T: Actor {
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


pub trait State: Any + Send + 'static {}

impl<T> State for T where T: Any + Send + 'static {}

pub trait Arg: Any + Send + 'static {}

impl<T> Arg for T where T: Any + Send + 'static {}

#[cfg(test)]
mod actor_test {
    use std::any::Any;
    use std::time::Duration;

    use anyhow::Ok;
    use async_trait::async_trait;
    use serde::{Deserialize, Serialize};
    use tracing::info;

    use actor_derive::EmptyCodec;

    use crate::actor::{Actor, CodecMessage, Message};
    use crate::actor::context::{ActorContext, Context};
    use crate::actor_ref::{ActorRef, ActorRefExt, SerializedActorRef, TActorRef};
    use crate::decoder::MessageDecoder;
    use crate::ext::encode_bytes;
    use crate::message::MessageRegistration;
    use crate::message::terminated::WatchTerminated;
    use crate::props::Props;
    use crate::provider::{ActorRefFactory, TActorRefProvider};
    use crate::system::ActorSystem;
    use crate::user_message_decoder;

    #[tokio::test]
    async fn test_death_watch() -> anyhow::Result<()> {
        #[derive(Debug)]
        struct TestActor;

        impl Actor for TestActor {
            type S = ();
            type A = usize;

            fn pre_start(&self, context: &mut ActorContext, arg: Self::A) -> anyhow::Result<Self::S> {
                info!("actor {} pre start", context.myself);
                for _ in 0..3 {
                    let n = arg - 1;
                    if n > 0 {
                        context.actor_of(TestActor, arg - 1, Props::default(), None)?;
                    }
                }
                Ok(())
            }

            fn post_stop(&self, context: &mut ActorContext, state: &mut Self::S) -> anyhow::Result<()> {
                info!("actor {} post stop",context.myself);
                Ok(())
            }
        }

        let system = ActorSystem::new("game".to_string(), "127.0.0.1:12121".parse()?, MessageRegistration::new())?;
        let actor = system.actor_of(TestActor, 3, Props::default(), None)?;
        tokio::time::sleep(Duration::from_secs(1)).await;
        system.stop(&actor);
        tokio::time::sleep(Duration::from_secs(3)).await;
        Ok(())
    }

    #[tokio::test]
    async fn test_watch() -> anyhow::Result<()> {
        struct TestActor;

        impl Actor for TestActor {
            type S = ();
            type A = ();

            fn pre_start(&self, context: &mut ActorContext, arg: Self::A) -> anyhow::Result<Self::S> {
                Ok(())
            }
        }

        #[derive(Debug, Serialize, Deserialize)]
        struct WatchActorTerminate {
            watch: SerializedActorRef,
        }

        impl WatchTerminated for WatchActorTerminate {
            fn watch_actor(&self, system: &ActorSystem) -> ActorRef {
                system.provider().resolve_actor_ref(&self.watch.path)
            }
        }

        impl CodecMessage for WatchActorTerminate {
            fn into_any(self: Box<Self>) -> Box<dyn Any> {
                self
            }

            fn decoder() -> Option<Box<dyn MessageDecoder>> where Self: Sized {
                Some(user_message_decoder!(WatchActorTerminate, TestActor))
            }

            fn encode(&self) -> Option<anyhow::Result<Vec<u8>>> {
                Some(encode_bytes(self))
            }
        }

        impl Message for WatchActorTerminate {
            type T = TestActor;

            fn handle(self: Box<Self>, context: &mut ActorContext, state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
                info!("{} watch actor {} terminate", context.myself, self.watch);
                Ok(())
            }
        }

        #[derive(Debug, EmptyCodec)]
        struct WatchFor {
            actor: ActorRef,
        }

        impl Message for WatchFor {
            type T = TestActor;

            fn handle(self: Box<Self>, context: &mut ActorContext, state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
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
            type T = TestActor;

            fn handle(self: Box<Self>, context: &mut ActorContext, state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
                info!("{} unwatch {}", context.myself, self.actor);
                context.unwatch(&self.actor);
                Ok(())
            }
        }

        let system1 = ActorSystem::new("game".to_string(), "127.0.0.1:12121".parse().unwrap(), MessageRegistration::new())?;
        let system2 = ActorSystem::new("game".to_string(), "127.0.0.1:12122".parse().unwrap(), MessageRegistration::new())?;
        let system1_actor = system1.actor_of(TestActor, (), Props::default(), None)?;
        let system2_actor1 = system2.actor_of(TestActor, (), Props::default(), None)?;
        let system2_actor2 = system2.actor_of(TestActor, (), Props::default(), None)?;
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
        struct TestActor;

        impl Actor for TestActor {
            type S = ();
            type A = ();

            fn pre_start(&self, context: &mut ActorContext, arg: Self::A) -> anyhow::Result<Self::S> {
                Ok(())
            }
        }
        #[derive(EmptyCodec)]
        struct LocalMessage;

        impl Message for LocalMessage {
            type T = TestActor;

            fn handle(self: Box<Self>, context: &mut ActorContext, state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
                Ok(())
            }
        }
    }
}
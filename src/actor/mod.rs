use std::any::Any;
use std::fmt::{Debug, Formatter};
use anyhow::anyhow;

use async_trait::async_trait;
use futures::future::LocalBoxFuture;
use futures::stream::FuturesUnordered;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use tracing_subscriber::fmt::writer::BoxMakeWriter;
use url::quirks::search;
use crate::actor;

use crate::actor::context::ActorContext;

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

#[async_trait(? Send)]
pub trait Message: Any + Send + 'static {
    type T: Actor;
    async fn handle(self: Box<Self>, context: &mut ActorContext, state: &mut <Self::T as Actor>::S) -> anyhow::Result<()>;
}

#[async_trait(? Send)]
pub trait LocalMessage: Message {}

impl<T> LocalMessage for T where T: Message {}

pub trait RemoteMessage: Message {
    fn decoder() -> Box<dyn MessageDecoder>;
    fn encode(&self) -> anyhow::Result<Vec<u8>>;
}

#[async_trait(? Send)]
pub trait SystemMessage: Any + Send + 'static {
    fn decoder() -> Box<dyn MessageDecoder> where Self: Sized;
    fn encode(&self) -> anyhow::Result<Vec<u8>>;
    async fn handle(self: Box<Self>, context: &mut ActorContext) -> anyhow::Result<()>;
}

#[derive(Debug)]
pub enum DynamicMessage {
    UserLocal(BoxedMessage),
    UserRemote(BoxedMessage),
    System(BoxedMessage),
}

pub(crate) struct BoxedMessage {
    name: &'static str,
    inner: Box<dyn Any + Send + 'static>,
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

    pub fn new<M>(message: M) -> Self where M: Any + Send + 'static {
        let name = std::any::type_name::<M>();
        BoxedMessage {
            name,
            inner: Box::new(message),
        }
    }
}

impl DynamicMessage {
    pub fn local<M>(message: M) -> Self where M: LocalMessage {
        DynamicMessage::UserLocal(BoxedMessage::new(UserDelegate::new(message)))
    }

    pub fn remote<M>(message: M) -> Self where M: RemoteMessage {
        DynamicMessage::UserRemote(BoxedMessage::new(UserDelegate::new(message)))
    }

    pub fn system<M>(message: M) -> Self where M: SystemMessage {
        DynamicMessage::System(BoxedMessage::new(SystemDelegate::new(message)))
    }

    pub(crate) fn name(&self) -> &'static str {
        match self {
            DynamicMessage::UserLocal(m) => { m.name() }
            DynamicMessage::UserRemote(m) => { m.name() }
            DynamicMessage::System(m) => { m.name() }
        }
    }

    pub(crate) fn downcast<T>(self) -> anyhow::Result<MessageDelegate<T>> where T: Actor {
        let name = self.name();
        let delegate = match self {
            DynamicMessage::UserLocal(m) => {
                m.inner.downcast::<UserDelegate<T>>().map(|m| MessageDelegate::User(m))
            }
            DynamicMessage::UserRemote(m) => {
                m.inner.downcast::<UserDelegate<T>>().map(|m| MessageDelegate::User(m))
            }
            DynamicMessage::System(m) => {
                m.inner.downcast::<SystemDelegate>().map(|m| MessageDelegate::System(m))
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

pub trait MessageDecoder {
    fn decode(&self, bytes: &[u8]) -> anyhow::Result<DynamicMessage>;
}

#[macro_export]
macro_rules! user_message_decoder {
    ($message:ident, $actor:ident) => {
        {
            struct D;
            impl MessageDecoder for D {
                fn decode(&self, bytes: &[u8]) -> anyhow::Result<crate::actor::DynamicMessage> {
                    let message: $message = crate::ext::decode_bytes(bytes)?;
                    let message = crate::actor::UserDelegate::<$actor>::new(message);
                    Ok(message.into())
                }
            }
            Box::new(D)
        }
    };
}

#[macro_export]
macro_rules! system_message_decoder {
    ($message:ident) => {
        {
            struct D;
            impl MessageDecoder for D {
                fn decode(&self, bytes: &[u8]) -> anyhow::Result<crate::actor::DynamicMessage> {
                    let message: $message = crate::ext::decode_bytes(bytes)?;
                    let message = crate::actor::SystemDelegate::new(message);
                    Ok(message.into())
                }
            }
            Box::new(D)
        }
    };
}

pub(crate) struct UserDelegate<T> where T: Actor {
    pub(crate) name: &'static str,
    pub(crate) message: Box<dyn Message<T=T>>,
}

impl<T> Debug for UserDelegate<T> where T: Actor {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MessageDelegate")
            .field("message", &"..")
            .finish()
    }
}

impl<T> UserDelegate<T> where T: Actor {
    pub fn new<M>(message: M) -> Self where M: Message<T=T> {
        Self {
            name: std::any::type_name::<M>(),
            message: Box::new(message),
        }
    }
}

#[async_trait(? Send)]
impl<T> Message for UserDelegate<T> where T: Actor + Send + 'static {
    type T = T;

    async fn handle(self: Box<Self>, context: &mut ActorContext, state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
        self.message.handle(context, state).await
    }
}

impl<T> Into<DynamicMessage> for UserDelegate<T> where T: Actor {
    fn into(self) -> DynamicMessage {
        DynamicMessage::UserLocal(BoxedMessage {
            name: self.name,
            inner: Box::new(self),
        })
    }
}

pub(crate) struct SystemDelegate {
    pub(crate) name: &'static str,
    pub(crate) message: Box<dyn SystemMessage>,
}

impl SystemDelegate where {
    pub fn new<M>(message: M) -> Self where M: SystemMessage {
        Self {
            name: std::any::type_name::<M>(),
            message: Box::new(message),
        }
    }
}

impl Into<DynamicMessage> for SystemDelegate {
    fn into(self) -> DynamicMessage {
        DynamicMessage::System(BoxedMessage {
            name: self.name,
            inner: Box::new(self),
        })
    }
}

pub(crate) enum MessageDelegate<T> where T: Actor {
    User(Box<UserDelegate<T>>),
    System(Box<SystemDelegate>),
}

impl<T> MessageDelegate<T> where T: Actor {
    pub(crate) fn name(&self) -> &'static str {
        match self {
            MessageDelegate::User(m) => { m.name }
            MessageDelegate::System(m) => { m.name }
        }
    }
}

pub trait State: Any + 'static {}

impl<T> State for T where T: Any + 'static {}

pub trait Arg: Any + Send + 'static {}

impl<T> Arg for T where T: Any + Send + 'static {}

#[cfg(test)]
mod actor_test {
    use std::time::Duration;
    use anyhow::__private::kind::TraitKind;

    use anyhow::Ok;
    use async_trait::async_trait;
    use futures::{FutureExt, TryFutureExt};
    use futures::future::LocalBoxFuture;
    use futures::stream::FuturesUnordered;
    use mlua::Lua;
    use mlua::prelude::LuaFunction;
    use tracing::info;

    use crate::actor::{Actor, LocalMessage, Message};
    use crate::actor::context::ActorContext;
    use crate::props::Props;
    use crate::provider::ActorRefFactory;
    use crate::system::ActorSystem;

    #[tokio::test]
    async fn test_death_watch() -> anyhow::Result<()> {
        #[derive(Debug)]
        struct TestActor;

        impl Actor for TestActor {
            type S = ();
            type A = usize;

            fn pre_start(&self, ctx: &mut ActorContext, arg: Self::A) -> anyhow::Result<Self::S> {
                info!("actor {} pre start", ctx.myself);
                for _ in 0..3 {
                    let n = arg - 1;
                    if n > 0 {
                        ctx.actor_of(TestActor, arg - 1, Props::default(), None)?;
                    }
                }
                Ok(())
            }

            fn post_stop(&self, ctx: &mut ActorContext, state: &mut Self::S) -> anyhow::Result<()> {
                info!("actor {} post stop",ctx.myself);
                Ok(())
            }
        }

        let system = ActorSystem::new("game".to_string(), "127.0.0.1:12121".parse()?)?;
        let actor = system.actor_of(TestActor, 3, Props::default(), None)?;
        tokio::time::sleep(Duration::from_secs(1)).await;
        system.stop(&actor);
        tokio::time::sleep(Duration::from_secs(3)).await;
        Ok(())
    }

    #[tokio::test]
    async fn test_lua() -> anyhow::Result<()> {
        struct LuaActor;

        struct State {
            lua: Lua,
        }

        struct LuaMessage;

        #[async_trait(? Send)]
        impl Message for LuaMessage {
            type T = LuaActor;

            async fn handle(self: Box<Self>, context: &mut ActorContext, state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
                let handle: LuaFunction = state.lua.globals().get("handle")?;
                let async_handle = handle.call_async::<_, ()>(()).map_err(anyhow::Error::from);
                Ok(())
            }
        }

        impl Actor for LuaActor {
            type S = State;
            type A = ();

            fn pre_start(&self, ctx: &mut ActorContext, arg: Self::A) -> anyhow::Result<Self::S> {
                let code = r#"
                function handle()
                    print("hello world")
                end
                "#;
                let lua = Lua::new();
                lua.load(&*code).exec()?;
                let state = State {
                    lua,
                };
                Ok(state)
            }
        }
        Ok(())
    }
}

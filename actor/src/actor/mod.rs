use std::any::Any;
use std::fmt::{Debug, Formatter};

use anyhow::anyhow;
use async_trait::async_trait;
use serde::de::DeserializeOwned;

use crate::actor::context::ActorContext;
use crate::decoder::MessageDecoder;
use crate::delegate::MessageDelegate;
use crate::delegate::system::SystemDelegate;
use crate::delegate::user::UserDelegate;

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

#[async_trait(? Send)]
pub trait Message: CodecMessage {
    type T: Actor;

    async fn handle(self: Box<Self>, context: &mut ActorContext, state: &mut <Self::T as Actor>::S) -> anyhow::Result<()>;
}

#[async_trait(? Send)]
pub trait SystemMessage: CodecMessage {
    async fn handle(self: Box<Self>, context: &mut ActorContext) -> anyhow::Result<()>;
}

#[derive(Debug)]
pub enum DynamicMessage {
    User(BoxedMessage),
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

    pub fn system<M>(message: M) -> Self where M: SystemMessage {
        let delegate = SystemDelegate::new(message);
        DynamicMessage::System(BoxedMessage::new(delegate.name, delegate))
    }

    pub(crate) fn name(&self) -> &'static str {
        match self {
            DynamicMessage::User(m) => { m.name() }
            DynamicMessage::System(m) => { m.name() }
        }
    }

    pub(crate) fn downcast<T>(self) -> anyhow::Result<MessageDelegate<T>> where T: Actor {
        let name = self.name();
        let delegate = match self {
            DynamicMessage::User(m) => {
                m.inner.into_any().downcast::<UserDelegate<T>>().map(|m| MessageDelegate::User(m))
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


pub trait State: Any + 'static {}

impl<T> State for T where T: Any + 'static {}

pub trait Arg: Any + Send + 'static {}

impl<T> Arg for T where T: Any + Send + 'static {}

#[cfg(test)]
mod actor_test {
    use std::any::Any;
    use std::time::Duration;

    use anyhow::Ok;
    use async_trait::async_trait;
    use futures::TryFutureExt;
    use mlua::Lua;
    use mlua::prelude::LuaFunction;
    use tracing::info;
    use actor_derive::Codec;

    use crate::actor::{Actor, CodecMessage, Message};
    use crate::actor::context::ActorContext;
    use crate::decoder::MessageDecoder;
    use crate::message::MessageRegistration;
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

        let system = ActorSystem::new("game".to_string(), "127.0.0.1:12121".parse()?, MessageRegistration::new())?;
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

        impl CodecMessage for LuaMessage {
            fn into_any(self: Box<Self>) -> Box<dyn Any> {
                self
            }

            fn decoder() -> Option<Box<dyn MessageDecoder>> where Self: Sized {
                None
            }

            fn encode(&self) -> Option<anyhow::Result<Vec<u8>>> {
                None
            }
        }
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
        #[derive(Codec)]
        struct LocalMessage;

        #[async_trait(? Send)]
        impl Message for LocalMessage {
            type T = TestActor;

            async fn handle(self: Box<Self>, context: &mut ActorContext, state: &mut <Self::T as Actor>::S) -> anyhow::Result<()> {
                Ok(())
            }
        }
    }
}
